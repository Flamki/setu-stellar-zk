//! # Withdrawal flow (prototype)
//!
//! ## Current behavior
//! The withdrawal recipient is expected to submit the withdrawal transaction
//! themselves. The recipient address is therefore visible on-chain as the
//! transaction source, and the recipient pays the Soroban resource fees
//! directly. This leaks metadata that is *not* covered by the ZK proof:
//! the link between the recipient account and the withdrawal event, the
//! timing of the withdrawal, and the fee payer identity.
//!
//! ## Relayer options considered for Soroban
//! 1. **Third-party relayer submits the tx.** A relayer account (or a pool of
//!    relayer accounts) signs and pays fees for the withdrawal transaction.
//!    The recipient address is passed as a contract argument instead of being
//!    the tx source. Privacy gain: recipient is decoupled from the tx source.
//!    Limit: the relayer learns the recipient address and the withdrawal
//!    timing, so privacy is only as good as the relayer's honesty and the
//!    anonymity set of relayer-submitted withdrawals.
//! 2. **Fee abstraction / sponsored fees.** Soroban supports fee-bump
//!    transactions, where a sponsor account pays the fee while the recipient
//!    (or relayer) provides the inner transaction. Privacy gain: fee payer is
//!    decoupled from the withdrawal. Limit: the inner tx source is still the
//!    recipient unless combined with option 1, so this alone does not hide
//!    the recipient.
//! 3. **Relayer with fee paid from the withdrawn value.** The proof commits
//!    to a fee amount, and the contract transfers `value - fee` to the
//!    recipient and `fee` to the relayer. Privacy gain: relayer is
//!    incentivized without an out-of-band payment. Limit: the fee amount and
//!    relayer address are public, and the circuit must be extended to bind
//!    the fee, which is not implemented here.
//!
//! ## Decisions (design only, not yet implemented)
//! - **Recipient address**: passed as a public contract argument, not derived
//!   from the proof. The proof binds the nullifier and commitment; the
//!   recipient is intentionally *not* bound by the current circuit.
//! - **Fee payment**: a relayer account pays the Soroban resource fee. The
//!   relayer is compensated out-of-band in this prototype. On-chain fee
//!   deduction from the withdrawn value is a follow-up task and requires a
//!   circuit change.
//! - **Replay protection**: the nullifier is the replay guard. The contract
//!   must reject a nullifier that has already been spent. This file only
//!   produces the SNARK input; nullifier storage and checking live in the
//!   contract and are a follow-up task.
//!
//! ## Privacy gains and limits
//! - Gain: with a relayer, the recipient is not the tx source, so the
//!   recipient account is not directly linked to the withdrawal tx.
//! - Limit: the relayer sees the recipient address and timing. A single
//!   relayer is a central point of metadata leakage.
//! - Limit: the current circuit does not bind the recipient or the fee, so
//!   the relayer could redirect funds if the contract does not enforce the
//!   recipient argument correctly.
//! - Limit: this is a prototype. No relayer flow is implemented in this
//!   module yet; only the SNARK input for a recipient-signed withdrawal is
//!   produced.
//!
//! ## Follow-up implementation tasks
//! - [ ] Add a `relayer` argument to the withdrawal contract entrypoint and
//!       route the fee to it.
//! - [ ] Extend the circuit to bind `recipient` and `fee` as public inputs.
//! - [ ] Add nullifier storage and replay rejection in the contract.
//! - [ ] Add tests covering relayer-submitted withdrawals and replay.
//! - [ ] Document relayer trust assumptions in the README.

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
    /// NOTE: This prototype assumes the recipient signs and pays for the
    /// withdrawal transaction directly. The recipient address is not part of
    /// the SNARK input and is not bound by the proof. See the module-level
    /// docs for the relayer design and its privacy limits. No relayer flow
    /// is implemented here yet.
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
