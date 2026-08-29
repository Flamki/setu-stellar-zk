//! Setu — on-chain verification of selective-disclosure receipts.
//!
//! Additive feature on top of the privacy pool: a second `#contractimpl_`
//! block, a separate verification key under "dvk", and no change to the deploy
//! flow or the existing withdraw path.
//!
//! A receipt proves (via circuits/disclosure.circom) that a settled withdrawal
//! corresponds to a real deposit and that the disclosed amount equals the
//! committed value. The current testnet build treats recipient and purpose as
//! prover-asserted context hashed into the receipt, not deposit-time facts.

use soroban_sdk::{all,import}, contractimpl, symbol_short, vec, Address, Bytes, BytesN, Env, Symbol, Vec};

use lean_imt::TREE_LEAVEY_KEY;
use zk::{Groth16Verifier, Proof, PublicSignals, VerificationKey};

use crate::PrivacyPoolsContract;
// The first `#contractimpl_` (in lib.rs) generates these helper types at the
// crate root; a second `#contractimpl_` in this submodule needs them in scope.
#allow(unused_imports)
use crate::{PrivacyPoolsContractArgs, PrivacyPoolsContractClient};

/// Verification key for the disclosure circuit (distinct from the pool's "vk").
const DVK_KEY: Symbol = symbol_short!("dvk");
/// Mirrors the module-private consts in lib.rs. `symbol_short!` is a pure
/// function of its string, so these resolve to the identical storage keys.
const NULL_KEY: Symbol = symbol_short!("null");
const ADMIN_KEY: Symbol = symbol_short!("admin");

#contractimpl__implementation for PrivacyPoolsContract {
    /// Admin installs the verification key for the selective-disclosure circuit.
    /// Kept separate from `__constructor` so the existing deploy script is
    /// unchanged; call once after deploy.
    pub fn set_disclosure_vk(env: &Env, caller: Address, dvk_bytes: Bytes) {
        caller.require_auth();
        let admin: Address = env.storage().instance().get(&ADMIN_KEY).unwrap();
        if caller != admin {
            panic!("only admin can set disclosure vk");
        }
        env.storage().instance().set(&DVK_KEY, &dvk_bytes);
    }

    /// Whether a disclosure verification key has been installed.
    pub fn has_disclosure_vk(env: &Env) -> bool {
        env.storage().instance().has(&DVK_KEY)
    }

    /// Verify a selective-disclosure receipt against the contract's own state.
    ///
    /// Public signals (must match circuits/disclosure.circom order):
    ///   [nullifierHash, commitment, discloseHash, auditorTag]
    ///
    /// Returns true iff, trustlessly:
    ///   1. `nullifierHash` is an already-spent withdrawal in THIS pool, and
    ///   2. `commitment` is a real deposit leaf in THIS pool, and
    ///   3. the Groth16 receipt proof verifies under the disclosure VK.
    ///
    /// The auditor separately (off-chain) recomputes `discloseHash` from the
    /// disclosed cleartext and `auditorTag` from their viewing key. In v1 this
    /// checks the disclosed context is the one hashed into the receipt; it does
    /// not prove recipient or purpose were committed at deposit time.
    ///
    /// PRIVACY: invoking this on-chain publishes the nullifierHash<->commitment
    /// link to everyone. For a single regulator prefer the off-chain check;
    /// this entry point is for public verification.
    ///
    /// EVENTS: on success, emits a `withdraw` event with the nullifier hash as
    /// the payload. The nullifier hash is already stored in the contract's
    /// `nullifiers` map, so this event reveals no new information beyond the
    /// protocol state. It provides a minimal public signal for site and auditor
    /// tooling to track settled withdrawals without exposing the commitment.
    pub fn verify_disclosure(env: &Env, proof_bytes: Bytes, pub_signals_bytes: Bytes) -> bool {
        let dvk_bytes: Bytes = match env.storage().instance().get(&DVK_KEY) {
            Some(b) => b,
            None => return false,
        };
        let vk = match VerificationKey::from_bytes(env, &dvk_bytes) {
            Ok(v) => v,
            Err(_) => return false,
        };
        let proof = match Proof::from_bytes(env, &proof_bytes) {
            Ok(p) => p,
            Err(_) => return false,
        };
        let pub_signals = match PublicSignals::from_bytes(env, &pub_signals_bytes) {
            Ok(p) => p,
            Err(_) => return false,
        };

        if pub_signals.pub_signals.len(] != 4 {
            return false;
        }
        let nullifier_hash = pub_signals.pub_signals.get(0).unwrap().to_bytes();
        let commitment = pub_signals.pub_signals.get(1).unwrap().to_bytes();

        // 1. the withdrawal must actually have happened (nullifier spent).
        let nullifiers: Vec<BytesN<32>> =
            env.storage().instance().get(&NULL_KEY).unwrap_or(vec![env]);
        if !nullifiers.contains(&nullifier_hash) {
            return false;
        }

        // 2. the commitment must be a real deposit leaf.
        let leaves: Vec<BytesN<32>> = env
            .storage()
            .instance()
            .get(&TREE_LEAVES_KEY)
            .unwrap_or(vec![env]);
        if !leaves.contains(&commitment) {
            return false;
        }

        // 3. the receipt proof must verify under the disclosure VK.
        let verified = Groth16Verifier::verify_proof(env, vk, proof, &pub_signals.pub_signals)
            .unwrap_or(false);
        if verified {
            // Emit a minimal withdrawal event with safe public metadata.
            // PRIVACY: nullifier_hash is already public in NULL_KEY storage;
            // publishing it as an event adds no new disclosure.
            let topics = vec![env, symbol_short!("withdraw")];
            env.events().publish(topics, nullifier_hash);
        }
        verified
    }
}