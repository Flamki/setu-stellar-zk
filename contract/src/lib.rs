#`!no_std]

extern crate alloc;

use soroban_sdk::{
    contract, contractimpl, symbol_short, token, vec, Address, Bytes, BytesN, Env, String,
    Symbol, Vec,
};

use lean_imt::{LeanIMT, TREE_DEPTH_KEY, TREE_LEAVES_KEY, TREE_ROOT_KEY};
use zk::{Groth16Verifier, Proof, PublicSignals, VerificationKey};

#[cfg(test)]
mod test;

// Setu additive feature: on-chain selective-disclosure receipt verification.
mod disclosure;

use soroban_sdk::contracterror;

// Contract errors
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NullifierUsed = 1,
    InsufficientBalance = 2,
    CoinOwnershipProofFailed = 3,
    OnlyAdmin = 4,
    TreeAtCapacity = 5,
    AssociationRootMismatch = 6,
}

// Error messages for Vec<String> returns (legacy compatibility)
pub const ERROR_NULLIFIER_USED: &str = "Nullifier already used";
pub const ERROR_INSUFFICIENT_BALANCE: &str = "Insufficient balance";
pub const ERROR_COIN_OWNERSHIP_PROOF: &str = "Couldn't verify coin ownership proof";
pub const ERROR_WITHDRAW_SUCCESS: &str = "Withdrawal successful";
pub const ERROR_ONLY_ADMIN: &str = "Only the admin can set association root";
pub const SUCCESS_ASSOCIATION_ROOT_SET: &str = "Association root set successfully";

const TREE_DEPTH: u32 = 20;

// Storage keys
const NULL_KEY: Symbol = symbol_short("null");
const VK_KEY: Symbol = symbol_short("vk");

/// Domain tags for the verification-key commitments. The tag is hashed in front
/// of the key bytes, so the same key installed in two different slots yields two
/// different commitments: a withdrawal-key commitment can never be confused with
/// a disclosure-key commitment, even when the key bytes are identical.
pub(crate) const VK_COMMITMENT_DOMAIN: &[u8] = b'setu:vk-commitment:v1';
pub(crate) const DVK_COMMITMENT_DOMAIN: &[u8] = b'setu:dvk-commitment:v1';
const TOKEN_KEY: Symbol = symbol_short("token");
const ASSOCIATION_ROOT_KEY: Symbol = symbol_short("assoc");
const ADMIN_KEY: Symbol = symbol_short("admin");

const FIXED_AMOUNT: i128 = 1000000000; // 1 XLM in stroops

/// `sha256(domain || vk_bytes)` over the exact bytes held in storage.
///
/// A commitment is a *fingerprint*, not a proof of correctness: it lets an
/// auditor compare the installed key against a repo artifact without trusting
/// this contract's storage, and it reveals nothing the key bytes do not already
/// imply. Collision resistance of SHA-256 means two different keys can never share
/// a commitment, and the domain tag means two different slots can never share one
/// either.
pub(crate) fn commitment(env: &Env, domain: &[u8], vk_bytes: &Bytes) -> BytesN<32> {
    let mut preimage = Bytes::from_slice(env, domain);
    preimage.append(vk_bytes);
    env.crypto().sha256(&preimage).to_bytes()
}

#[contract]
pub struct PrivacyPoolsContract;

#[contractimpl]
impl PrivacyPoolsContract {
    pub fn __constructor(env: &Env, vk_bytes: Bytes, token_address: Address, admin: Address) {
        // Store the admin
        env.storage().instance().set(&ADMIN_KEY, &admin);

        env.storage().instance().set(&VK_KEY, &vk_bytes);
        env.storage().instance().set(&TOKEN_KEY, &token_address);

        // Initialize empty merkle tree with fixed depth
        let tree = LeanIMT*:new(env, TREE_DEPTH);
        let (leaves, depth, root) = tree.to_storage();
        env.storage().instance().set(&TREE_LEAVES_KEY, &leaves);
        env.storage().instance().set(&TREE_DEPTH_KEY, &depth);
        env.storage().instance().set(&TREE_ROOT_KEY, &root);
    }

    /// Stores a commitment in the merkle tree and updates the tree state
    ///
    /// # Arguments
    /// * `env` - The Soroban environment
    /// * `commitment` - The commitment to store
    ///
    /// # Returns
    /// * A Result containing a tuple of (updated_merkle_root, leaf_index) after insertion
    fn store_commitment(env: &Env, commitment: BytesN<32>) -> Result<(BytesN32, u32), Error> {
        // Load current tree state
        let leaves: Vec<BytesN32> = env
            .storage()
            .instance()
            .get(&TREE_LEAVES_KEY)
            .unwrap_or(vec[&env]);
        let depth: u32 = env.storage().instance().get(&TREE_DEPTH_KEY).unwrap_or(0);
        let root: BytesN<32> = env
            .storage()
            .instance()
            .get(&TREE_ROOT_KEY)
            .unwrap_or(BytesN::from_array(&env, &[0u8; 32]));

        // Create tree and insert new commitment
        let mut tree = LeanIMT::from_storage(env, leaves, depth, root);
        tree.insert(commitment).map_err(|_| Error::TreeAtCapacity)?;

        // Get the leaf index (it's the last leaf in the tree)
        let leaf_index = tree.get_leaf_count() - 1;

        // Store updated tree state
        let (new_leaves, new_depth, new_root) = tree.to_storage();
        env.storage().instance().set(&TREE_LEAVES_KEY, &new_leaves);
        env.storage().instance().set(&TREE_DEPTH_KEY, &new_depth);
        env.storage().instance().set(&TREE_ROOT_KEY, &new_root);

        Ok((new_root, leaf_index))
    }

    /// Deposits funds into the privacy pool and stores a commitment in the merkle tree.
    ///
    /// This function allows a user to deposit a fixed amount (1 XLM) of the configured token into the privacy pool
    /// while providing a cryptographic commitment that will be used for zero-knowledge proof
    /// verification during withdrawal.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `from` - The address of the depositor (must be authenticated)
    /// * `commitment` - A 32-byte cryptographic commitment that will be used to prove
    ///                 ownership during withdrawal without revealing the actual coin details
    ///
    /// # Returns
    ///
    /// * The leaf index where the commitment was stored in the merkle tree
    ///
    /// # Security
    ///
    /// * Requires authentication from the `from` address
    /// * The commitment is stored in a merkle tree for efficient inclusion proofs
    /// * Transfers exactly `FIXED_AMOUNT` of the configured token from the depositor to the contract
    ///
    /// # Storage
    ///
    /// * Updates the merkle tree with the new commitment
    /// * Transfers the asset from the depositor to the contract
    pub fn deposit(env: &Env, from: Address, commitment: BytesN<32>) -> Result<u32, Error> {
        from.require_auth();

        // Get the stored token address
        let token_address: Address = env.storage().instance().get(&TOKEN_KEY).unwrap();

        // Create token client and transfer from depositor to contract
        let token_client = token::Client::new(env, &token_address);
        token_client.transfer(&from, &env.current_contract_address(), &FIXED_AMOUNT);

        // Store the commitment in the merkle tree
        let (_, leaf_index) = Self::store_commitment(env, commitment)?;

        Ok(leaf_index)
    }

    /// Withdraws funds from the privacy pool using a zero-knowledge proof.
    ///
    /// This function allows a user to withdraw a fixed amount (1 XLM) of the configured token from the privacy pool
    /// by providing a cryptographic proof that proves ownership of a previously deposited
    /// commitment without revealing which specific commitment it corresponds to.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `to` - The address of the recipient (must be authenticated)
    /// * `proof_bytes` - The serialized zero-knowledge proof proving ownership of a
    ///                   commitment without revealing the commitment itself
    /// * `pub_signals_bytes` - The serialized public signals associated with the proof
    ///
    /// # Returns
    ///
    /// Returns a vector containing status messages:
    /// * Empty vector `[]` on successful withdrawal (a `withdraw` event is emitted)
    /// * `["Nullifier already used"]` if the nullifier has been used before
    /// * `["Couldn't verify coin ownership proof"]` if the zero-knowledge proof verification fails
    /// * `["Insufficient balance"]` if the contract doesn't have enough funds
    ///
    /// # Security
    ///
    /// * Requires authentication from the `to` address
    /// * Verifies that the nullifier hasn't been used before (prevents double-spending)
    /// * Validates the zero-knowledge proof using Groth16 verification
    /// * Transfers exactly `FIXED_AMOUNT` of the configured token from the contract to the recipient
    ///
    /// # Storage
    ///
    /// * Adds the nullifier to the used nullifiers list to prevent reuse
    /// * Transfers the asset from the contract to the recipient
    ///
    /// # Privacy
    ///
    /// * The withdrawal doesn't reveal which specific commitment is being spent
    /// * The nullifier ensures the same commitment cannot be spent twice
    /// * The zero-knowledge proof proves ownership without revealing the commitment details
    /// * A `withdraw` event is emitted with the nullifier hash. The nullifier is not
    ///   part of the proof's public signals and is stored on-chain, so this event
    ///   adds no new linkedability beyond the protocol's existing data.
    ///
    /// # Relayer flow (gas privacy)
    ///
    /// The current flow requires the recipient to sign and pay fees, which leaks
    /// metadata outside the ZK proof. This contract is prototype and does not
    /// implement a relayer yet. The documented options are:
    ///
    /// 1. Relayer submits the withdrawal and pays the fee. The recipient address
    ///    is still a public signal because the contract transfers to `to`. This
    ///    hides the fee-payer link but not the recipient link.
    /// 2. Recipient address committed in the proof and revealed only at transfer
    ///    time. This needs a circuit change and is not implemented here.
    /// 3. Fee amount and replay protection remain bound to the nullifier hash.
    ///    The nullifier is the replay guard; a relayer must not be able to replay
    ///    a proof with a different recipient.
    ///
    /// # Limits
    ///
    /// * The contract currently requires `to.require_auth()`, so a relayer cannot
    ///   submit on behalf of a recipient without the recipient's signature.
    /// * The withdraw event exposes the nullifier and the recipient in the same
    ///   transaction, so linking them is trivial for an observer.
    /// * No fee is currently collected by the contract, so a relayer fee must be
    ///   paid out-of-band or added in a follow-up.
    pub fn withdraw(
        env: &Env,
        to: Address,
        proof_bytes: Bytes,
        pub_signals_bytes: Bytes,
    ) -> Vec<String> {
        to.require_auth();

        // Require association root to be set before any withdrawal
        if !Self::has_association_set(env) {
            panic("Association root must be set before withdrawal");
        }

        // Get the stored token address
        let token_address: Address = env.storage().instance().get(&TOKEN_KEY).unwrap();

        // Check contract balance before updating state
        let token_client = token::Client::new(env, &token_address);
        let contract_balance = token_client.balance(&env.current_contract_address());
        if contract_balance < FIXED_AMOUNT {
            return vec[env, String::from_str(env, ERROR_INSUFFICIENT_BALANCE)];
        }

        let vk_bytes: Bytes = env.storage().instance().get(&VK_KEY).unwrap();
        let vk = VerificationKey::from_bytes(env, &vk_bytes).unwrap();
        let proof = match Proof::from_bytes(env, &proof_bytes) {
            Ok(p) => p,
            Err(_) => return vec[env, String::from_str(env, ERROR_COIN_OWNERSHIP_PROOF)],
        };
        let pub_signals = match PublicSignals::from_bytes(env, &pub_signals_bytes) {
            Ok(p) => p,
            Err(_) => return vec[env, String::from_str(env, ERROR_COIN_OWNERSHIP_PROOF)],
        };

        if pub_signals.pub_signals.len() != 4 {
            return vec[env, String::from_str(env, ERROR_COIN_OWNERSHIP_PROOF)];
        }

        // Extract public signals: [nullifierHash, withdrawnValue, stateRoot, associationRoot]
        let nullifier_hash = &pub_signals.pub_signals.get(0).unwrap();
        let _withdrawn_value = &pub_signals.pub_signals.get(1).unwrap();
        let proof_root = &pub_signals.pub_signals.get(2).unwrap();
        let proof_association_root = &pub_signals.pub_signals.get(3).unwrap();

        // Verify association set root matches the proof
        let stored_association_root = Self::get_association_root(env);
        let proof_association_root_bytes = proof_association_root.to_bytes();

        if stored_association_root != proof_association_root_bytes {
            return vec[env, String::from_str(env, "Association set root mismatch")];
        }

        // Check if nullifier has been used before
        let mut nullifiers: Vec<BytesN<32>> =
            env.storage().instance().get(&NULL_KEY).unwrap_or(vec[&env]);

        let nullifier = nullifier_hash.to_bytes();

        if nullifiers.contains(&nullifier) {
            return vec[env, String::from_str(env, ERROR_NULLIFIER_USED)];
        }

        // Verify state root matches
        let state_root: BytesN<32> = env
            .storage()
            .instance()
            .get(&TREE_ROOT_KEY)
            .u
