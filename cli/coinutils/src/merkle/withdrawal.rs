//! Withdrawal SNARK input generation.
//!
//! # Relayer flow design (prototype)
//!
//! ## Problem
//! In the current prototype, the withdrawal recipient signs the Soroban
//! transaction and pays the network fee directly. This leaks metadata that is
//! *outside* the ZK proof: the recipient's Stellar account, the fee source, and
//! the timing of the withdrawal are all publicly linkable on-chain. The ZK
//! proof only hides the note (value/label/nullifier/secret), not the payer or
//! the destination.
//!
//! ## Relayer options for Soroban
//! 1. **Third-party relayer (fee-bump / sponsored tx).** A relayer account
//!    submits the withdrawal transaction and pays the fee. The recipient
//!    address is still present in the transaction as the withdrawal target,
//!    so this only hides *who paid*, not *who received*.
//! 2. **Relayer with a fresh recipient address per withdrawal.** The relayer
//!    submits the tx to a one-time address derived off-chain. This decouples
//!    the funding account from the final recipient, at the cost of an extra
//!    transfer step and additional on-chain footprint.
//! 3. **Relayer + shielded recipient (not implemented here).** The withdrawal
//!    credits a shielded pool entry instead of a transparent account, so the
//!    final recipient is only revealed on a later, separate withdrawal. This
//!    is the strongest option but requires a shielded pool contract that does
//!    not exist in this prototype.
//!
//! ## Decisions (prototype scope)
//! - **Recipient address:** passed as a plain `Address` argument to the
//!   withdrawal entrypoint. The relayer does not learn the note preimage; it
//!   only forwards the SNARK input and the recipient. Recipient privacy is
//!   therefore *not* provided by this module.
//! - **Fee payment:** the relayer pays the Soroban resource fee. The recipient
//!   does not need a funded account to receive the withdrawal. Fee payment is
//!   not part of the SNARK input and is not proven.
//! - **Replay protection:** the nullifier is the on-chain replay guard. The
//!   contract must record spent nullifiers and reject duplicates. The relayer
//!   is untrusted for replay protection; it cannot forge a valid nullifier
//!   without the note secret.
//!
//! ## Privacy gains and limits
//! - **Gains:** the fee payer is decoupled from the recipient; the note
//!   preimage is never revealed to the relayer; withdrawal timing can be
//!   batched by the relayer.
//! - **Limits:** the recipient address is public on-chain; the withdrawal
//!   amount is public; the relayer sees the recipient and the timing; the
//!   relayer can censor or delay. No anonymity set is provided by this module
//!   alone.
//!
//! ## Follow-up implementation tasks
//! - [ ] Add a `relayer` entrypoint that accepts a signed withdrawal payload
//!       and a recipient `Address`, and pays the fee from the relayer account.
//! - [ ] Add on-chain nullifier set with spent-nullifier checks and tests.
//! - [ ] Add a one-time recipient address derivation helper and tests.
//! - [ ] Evaluate a shielded-pool recipient path once a pool contract exists.
//! - [ ] Add integration tests proving replay rejection and fee sponsorship.
//!
//! NOTE: This module only builds SNARK input. It does **not** implement the
//! relayer flow above; those are follow-up tasks. No privacy claim here is
//! backed by tests yet.

use crate::{
    config::TREE_DEPTH,
    crypto::{coin::generate_commitment, conversions::*},
    error::{CoinUtilsError, Result},
    types::{AssociationSetFile, CoinData, SnarkInput, StateFile},
};
use lean_imt::LeanIMT;
use soroban_sdk::{crypto::bls12_381::Fr as BlsScalar, Env};

/// Manager for handling coin withdrawal operations
pub struct WithdrawalManager;

impl WithdrawalManager {
    pub fn new() -> Self {
        Self
    }

    /// Withdraw a coin and generate SNARK input
    ///
    /// This produces the SNARK input only. It does not submit a transaction,
    /// does not pay fees, and does not implement relayer submission. See the
    /// module-level docs for the relayer design and its privacy limits.
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
