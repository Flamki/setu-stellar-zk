//! Withdrawal SNARK input construction for the CoinUtils CLI.
//!
//! # Prototype limits (read before relying on this module)
//!
//! This module only builds the witness/`SnarkInput` for a withdrawal proof.
//! It does **not** implement a relayer, does **not** submit transactions, and
//! does **not** provide any on-chain privacy guarantees by itself.
//!
//! ## Privacy context
//!
//! In the current prototype the withdrawal recipient signs and pays the
//! transaction fee directly. That leaks metadata (recipient account, fee
//! payer, timing, and any Soroban auth entries) *outside* the ZK proof, even
//! though the note itself stays hidden. A relayer flow is the intended
//! mitigation, but it is **not implemented here**.
//!
//! ## Relayer options considered for Soroban (design only, untested)
//!
//! 1. **Explicit relayer account**: recipient hands the proof to a relayer
//!    service; the relayer submits the withdrawal tx and pays the fee. The
//!    recipient address never appears in the tx. Requires the contract to
//!    accept the proof from any caller and to bind the payout to a value
//!    committed inside the proof (or a fresh one-time address).
//! 2. **Fee-in-note**: the note value is split into payout + relayer fee, so
//!    the relayer is compensated from the shielded pool. Requires the circuit
//!    to expose a fee field and the contract to enforce it.
//! 3. **Meta-transaction / sponsored auth**: keep the recipient as the
//!    logical signer but have the relayer cover fees via Soroban auth
//!    delegation. Still leaks the recipient as the auth signer unless the
//!    contract uses a one-time key.
//!
//! ## Open decisions (tracked as follow-ups, not resolved here)
//!
//! - How the recipient address is bound: committed in the note vs. supplied
//!   as a fresh one-time address at withdrawal time.
//! - How the relayer fee is paid: out-of-band vs. fee-in-note.
//! - Replay protection: nullifier is already in the proof; the contract must
//!   still enforce single-use on-chain. Relayer identity must not be the only
//!   replay guard.
//!
//! ## Follow-up implementation tasks
//!
//! - [ ] Add a `relayer` / `fee` field to the withdrawal circuit and contract.
//! - [ ] Implement a relayer client that submits the withdrawal tx.
//! - [ ] Add contract-side nullifier replay protection tests.
//! - [ ] Add integration tests proving the recipient address is absent from
//!       the submitted transaction when a relayer is used.
//! - [ ] Update README to describe the relayer flow and its limits.
//!
//! No privacy claim in this file should be treated as verified until the
//! follow-up tests above exist and pass.

use crate::{
    config::TREE_DEPTH,
    crypto::{coin::generate_commitment, conversions::*},
    error::{CoinUtilsError, Result},
    types::{AssociationSetFile, CoinData, SnarkInput, StateFile},
};
use lean_imt::LeanIMT;
use soroban_sdk::{crypto::bls12_381::Fr as BlsScalar, Env};

/// Manager for handling coin withdrawal operations.
///
/// Note: this type only produces SNARK input. It does not submit transactions
/// and does not implement relaying. See the module-level docs for the relayer
/// design notes and prototype limits.
pub struct WithdrawalManager;

impl WithdrawalManager {
    pub fn new() -> Self {
        Self
    }

    /// Withdraw a coin and generate SNARK input.
    ///
    /// This builds the witness for the withdrawal proof from local state. It
    /// does not broadcast anything and does not hide the caller's identity:
    /// whoever submits the resulting proof on-chain is the fee payer and is
    /// visible outside the proof. A relayer flow is required to change that,
    /// and is not implemented here.
    pub fn withdraw_coin(
        &self,
        env: &Env,
        coin: &CoinData,
        state_file: &StateFile,
        association_set_file: Option<&AssociationSetFile>,
    ) -> Result<SnarkInput> {
        // Parse decimal string values to BlsScalar
        let value = decimal_string_to_bls_scalar(env, &coin.value)?;
        let nullifier = decimal_string_to_bls_scalar(env, &coin.nullifier)?;
        let secret = decimal_string_to_bls_scalar(env, &coin.secret)?;
        let label = decimal_string_to_bls_scalar(env, &coin.label)?;

        // Reconstruct the commitment to verify it matches
        let commitment = generate_commitment(
            env,
            value.clone(),
            label.clone(),
            nullifier.clone(),
            secret.clone(),
        );

        // Build merkle tree from state file using lean-imt
        let mut tree = LeanIMT::new(env, TREE_DEPTH);
        let mut commitment_index = None;

        for (index, commitment_str) in state_file.commitments.iter().enumerate() {
            let commitment_fr = decimal_string_to_bls_scalar(env, commitment_str).map_err(|e| {
                CoinUtilsError::InvalidDecimal(format!(
                    "Invalid commitment at index {}: {}",
                    index, e
                ))
            })?;

            // Convert BlsScalar to bytes and insert into lean-imt
            let commitment_bytes = lean_imt::bls_scalar_to_bytes(commitment_fr.clone());
            tree.insert(commitment_bytes)?;

            // Check if this is the commitment we're withdrawing
            if commitment_fr == commitment {
                commitment_index = Some(index);
            }
        }

        // Verify the commitment exists in the state
        let commitment_index =
            commitment_index.ok_or_else(|| CoinUtilsError::CommitmentNotFound)?;

        // Generate merkle proof using lean-imt
        let proof = tree
            .generate_proof(commitment_index as u32)
            .ok_or_else(|| CoinUtilsError::ProofGenerationFailed)?;
        let (siblings_scalars, _depth) = proof;

        // Convert siblings from BlsScalar to strings
        let siblings: Vec<BlsScalar> = siblings_scalars.iter().map(|s| s.clone()).collect();

        // Get the root from lean-imt
        let root_scalar = lean_imt::bytes_to_bls_scalar(&tree.get_root());

        // Handle association set
        let (association_root, label_index, label_siblings) =
            if let Some(association_set) = association_set_file {
                self.handle_association_set(env, association_set, &label)?
            } else {
                // No association set - use dummy values
                (
                    "0".to_string(),
                    "0".to_string(),
                    vec!["0".to_string(), "0".to_string()],
                )
            };

        let label_decimal = bls_scalar_to_decimal_string(&label);
        let value_decimal = bls_scalar_to_decimal_string(&value);
        let nullifier_decimal = bls_scalar_to_decimal_string(&nullifier);
        let secret_decimal = bls_scalar_to_decimal_string(&secret);
        let state_root_decimal = bls_scalar_to_decimal_string(&root_scalar);

        Ok(SnarkInput {
            withdrawn_value: crate::config::COIN_VALUE.to_string(),
            label: label_decimal,
            value: value_decimal,
            nullifier: nullifier_decimal,
            secret: secret_decimal,
            state_root: state_root_decimal,
            state_index: commitment_index.to_string(),
            state_siblings: siblings
                .into_iter()
                .map(|s| bls_scalar_to_decimal_string(&s))
                .collect(),
            association_root,
            label_index,
            label_siblings,
        })
    }

    /// Handle association set processing for withdrawal
    fn handle_association_set(
        &self,
        env: &Env,
        association_set: &AssociationSetFile,
        label: &BlsScalar,
    ) -> Result<(String, String, Vec<String>)> {
        use crate::config::ASSOCIATION_TREE_DEPTH;

        // Build association set merkle tree (depth 2)
        let mut association_tree = LeanIMT::new(env, ASSOCIATION_TREE_DEPTH);
        let mut label_index = None;

        for (index, label_str) in association_set.labels.iter().enumerate() {
            let label_fr = decimal_string_to_bls_scalar(env, label_str).map_err(|e| {
                CoinUtilsError::InvalidDecimal(format!(
                    "Invalid association label at index {}: {}",
                    index, e
                ))
            })?;

            // Convert BlsScalar to bytes and insert into association tree
            let label_bytes = lean_imt::bls_scalar_to_bytes(label_fr.clone());
            association_tree.insert(label_bytes)?;

            // Check if this is the label we're using
            if label_fr == *label {
                label_index = Some(index);
            }
        }

        // Verify the label exists in the association set
        let label_index = label_index.ok_or_else(|| CoinUtilsError::LabelNotFound)?;

        // Generate association set merkle proof
        let association_proof = association_tree
            .generate_proof(label_index as u32)
            .ok_or_else(|| CoinUtilsError::ProofGenerationFailed)?;
        let (association_siblings_scalars, _depth) = association_proof;

        let association_root_scalar = lean_imt::bytes_to_bls_scalar(&association_tree.get_root());
        let association_siblings: Vec<BlsScalar> = association_siblings_scalars
            .iter()
            .map(|s| s.clone())
            .collect();

        Ok((
            bls_scalar_to_decimal_string(&association_root_scalar),
            label_index.to_string(),
            association_siblings
                .into_iter()
                .map(|s| bls_scalar_to_decimal_string(&s))
                .collect(),
        ))
    }
}

impl Default for WithdrawalManager {
    fn default() -> Self {
        Self::new()
    }
}
