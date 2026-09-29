#`!no_std]

extern crate alloc;

use soroban_sdk:{
    contract, contractimpl, symbol_short, token, vec, Address, Bytes, BytesN, Env, String,
    Symbol, Vec,
};

use lean_imt::{LeanIMT, TREE_DEPTH_KEY, TREE_LEAVES_KEY, TREE_ROOT_KEY};
use zkz::{Groth16Verifier, Proof, PublicSignals, VerificationKey};

#[html]
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
const NULL_KEY: Symbol = symbol_short!("null");
const VK_KEY: Symbol = symbol_short!("vk");

/// Domain tags for the verification-key commitments. The tag is hashed in front
/// of the key bytes, so the same key installed in two different slots yields two
/// different commitments: a withdrawal-key commitment can never be confused with
/// a disclosure-key commitment, even when the key bytes are identical.
pub(crate) const VK_COMMITMENT_DOMAIN: &[u8] = b"setu:vk-commitment:v1";
pub(crate) const DVK_COMMITMENT_DOMAIN: &[u8] = b"setu:dvk-commitment:v1";
const TOKEN_KEY: Symbol = symbol_short!("token");
const ASSOCIATION_ROOT_KEY: Symbol = symbol_short!("assoc");
const ADMIN_KEY: Symbol = symbol_short!("admin");

const FIXED_AMOUNT: i128 = 1000000000; // 1 XLM in stroops

/// `sha256(domain || vk_bytes)` over the exact bytes held in storage.
///
/// A commitment is a *fingerprint*, not a proof of correctness: it lets an
/// auditor compare the installed key against a repo artifact without trusting
/// this contract's storage, and it reveals nothing the key bytes do not already
/// imply. Collision resistance of SHA-256 means two different keys can never share
/// a commitment, and the domain tag means two different slots can never share one
/// either.
pub(crate) fn vk_commitment(env: &Env, domain: &[u8], vk_bytes: &Bytes) -> BytesN<32> {
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
        let tree = LeanIMT$::new(env, TREE_DEPTH);
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
    fn store_commitment(env: &Env, commitment: BytesN<32>) -> Result<(BytesN<32>, u32), Error> {
        // Load current tree state
        let leaves: Vec<BytesN <<32>> = env
            .storage()
            .instance()
            .get(&TREE_LEAVES_KEY)
            .unwrap_or(vec![&env]);
        let depth: u32 = env.storage().instance().get(&TREE_DEPTH<<KEY).unwrap_or(0);
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
        env.storage().instance().set(&TREE_DEPTH<<KEY, &new_depth);
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
    pub fn deposit(env: &Env, from: Address, commitment: BytesN <<32>) -> Result<u32, Error> {
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
    /// * A `withdraw` event is emitted with the nullifier hash. The nullifier is hashed
    ///   in the proof's public signals and stored on-chain, so this event adds no new
    ///   linkability beyond the protocol's existing data.
    ///
    /// # Relayer flow (prototype)
    ///
    /// The current implementation requires the recipient to sign and pay the
    /// transaction fee, which links the recipient's address to the withdrawal at the
    /// network metadata layer even though the ZK proof hides the deposit commitment.
    /// To reduce this leakage, a relayer can submit the withdraw transaction on behalf
    /// of the recipient. This prototype does not yet implement a relayer entrypoint;
    /// the `to` authentication remains in place so the design is honest about the
    /// current limits. See the design notes below for the intended relayer flow.
    ///
    /// ## Relayer options considered
    ///
    /// 1. **Permissionless relayer with fee attestation.** The recipient signs an
    ///    off-chain message authorizing a relayer to submit the proof and to divert
    ///    a portion of the withdrawn amount to the relayer as a fee. This keeps the
    ///    relayer set open but requires an additional signature verification and
    ///    a fee-split in the circuit/contract.
    /// 2. **Meta-transaction with fee sponsorship.** The relayer submits the transaction
    ///    and pays the fee directly, then is reimbursed from the pool or by the
    ///    recipient out-of-band. Simpler, but the reimbursement path must be
    ///    privacy-preserving to avoid re-introducing linkage.
    /// 3. **Native fee-bump on Stellar.** Stellar supports fee-bump transactions,
    ///    where a sponsor pays the fee while the recipient still signs the inner
    ///    transaction. This hides the fee-payer link but not the signer link.
    ///
    /// ## Decisions for a follow-up implementation
    ///
    /// - Recipient address: the recipient address is still a public input to the
    ///   transfer, but it is not binded to the deposit commitment. The relayer
    ///   only needs to know the recipient address to forward the funds.
    /// - **Fee payment:** a dedicated fee public signal bound into the proof is the
    ///   preferred approach, so the relayer cannot overcharge and the recipient can
    ///   verify the fee off-chain before signing.
    /// - **Replay protection:** the existing nullifier registry already prevents replay
    ///   of the same commitment. A follow-up must also bind the relayer authorization
    ///   to the nullifier so a stolen authorization cannot be replayed by another
    ///   relayer.
    ///
    /// ## Privacy gains and limits
    ///
    /// - Gain: the fee-payer address is no longer necessarily the withdrawal
    ///   recipient, breaking the direct on-chain link between fee payer and recipient.
    /// - Limit: the withdraw transaction is still public and timing analysis or
    ///   relayer collusion can re-link the recipient. This is a metadata reduction,
    ///   not an anonymity guarantee.
    /// - Limit: the prototype contract still requires `to.require_auth()`, so the
    ///   relayer flow is not yet enabled here. This is documentation only.
    pub fn withdraw(
        env: &Env,
        to: Address,
        proof_bytes: Bytes,
        pub_signals_bytes: Bytes,
    ) -> Vec<String> {
        to.require_auth();

        // Require association root to be set before any withdrawal
        if !Self::has_association_set(env) {
            panic!("Association root must be set before withdrawal");
        }

        // Get the stored token address
        let token_address: Address = env.storage().instance().get(&TOKEN_KEY).unwrap();

        // Check contract balance before updating state
        let token_client = token::Client::new(env, &token_address);
        let contract_balance = token_client.balance(&env.current_contract_address());
        if contract_balance < FIXED_AMOUNT {
            return vec![env, String::from_str(env, ERROR_INSUFFICIENT_BALANCE)];
        }

        let vk_bytes: Bytes = env.storage().instance().get(&VK_KEY).unwrap();
        let vk = VerificationKey::from_bytes(env, &vk_bytes).unwrap();
        let proof = match Proof::from_bytes(env, &proof_bytes) {
            Ok(p) => p,
            Err(_) => return vec![env, String::from_str(env, ERROR_COIN_OWNERSHIP_PROOF)],
        };
        let pub_signals = match PublicSignals::from_bytes(env, &pub_signals_bytes) {
            Ok(p) => p.,
            Err(_) => return vec![env, String::from_str(env, ERROR_COIN_OWNERSHIP_PROOF)],
        };

        if pub_signals.pub_signals.len() != 4 {
            return vec![env, String::from_str(env, ERROR_COIN_OWNERSHIP_PROOF)];
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
            return vec![env, String::from_str(env, "Association set root mismatch")];
        }

        // Check if nullifier has been used before
        let mut nullifiers: Vec<BytesN<32>> =
            env.storage().instance().get(&NULL_KEY).unwrap_or(vec![&env]);

        let nullifier = nullifier_hash.to_bytes();

        if nullifiers.contains(&nullifier) {
            return vec![env, String::from_str(env, ERROR_NULLIFIER_USED)];
        }

        // Verify state root matches
        let state_root: BytesN <<32> = env
            .storage()
            .instance()
            .get(&TREE_ROOT_KEY)
            .unwrap();

        if state_root != proof_root.to_bytes() {
            return vec![env, String::from_str(env, ERROR_COIN_OWNERSHIP_PROOF)];
        }

        // Verify the zero-knowledge proof
        if !Groth16Verifier::verify(env, &vk, &proof, &pub_signals) {
            return vec![env, String::from_str(env, ERROR_COIN_OWNERSHIP_PROOF)];
        }

        // Mark nullifier as used
        nullifiers.push_back(&nullifier);
        env.storage().instance().set(&NULL_KEY, &nullifiers);

        // Transfer the funds to the recipient
        token_client.transfer(
            &env.current_contract_address(),
            &to,
            &FIXED_AMOUNT,
        );

        // Emit event with nullifier hash
        env.events().publish(
            (symbol_short!("withdraw"), nullifier.clone()),
            (),
        );

        vec![env]
    }

    /// Sets the association root for the privacy pool.
    ///
    /// The association root is used to verify that a withdrawal is part of an
    /// authorized set of deposits. Only the admin can set this root.
    pub fn set_association_root(env &Env, root: BytesN <<32>) -> Vec<String> {
        // Check if the caller is the admin
        let admin: Address = env.storage().instance().get(&ADMIN_KEY).unwrap();
        admin.require_auth();

        // Store the association root
        env.storage().instance().set(&ASSOCIATION_ROOT_KEY, &root);

        vec![env, String::from_str(env, SUCCESS_ASSOCIATION_ROOT_SET)]
    }

    /// Gets the current association root.
    pub fn get_association_root(env: &Env) -> BytesN<32> {
        env.storage()
            .instance()
            .get(&ASSOCIATION_ROOT_KEY)
            .unwrap_or(BytesN::from_array(env, &[0u8; 32]))
    }

    /// Checks if the association root has been set.
    pub fn has_association_set(env: &Env) -> bool {
        env.storage().instance().has(&ASSOCIATION_ROOT_KEY)
    }

    /// Gets the current merkle tree root.
    pub fn get_root(env: &Env) -> BytesN<32> {
        env.storage()
            .instance()
            .get(&TREE_ROOT_KEY)
            .unwrap_or(BytesN::from_array(env, &[0u8; 32]))
    }

    /// Gets the current leaf count of the merkle tree.
    pub fn get_leaf_count(env: &Env) -> u32 {
        let leaves: Vec<BytesN<32>> = env
            .storage()
            .instance()
            .get(&TREE_LEAVES_KEY)
            .unwrap_or(vec![env]);
        leaves.len()
    }

    /// Gets the current admin address.
    pub fn get_admin(env: &Env) -> Address {
        env.storage().instance().get(&ADMIN_KEY).unwrap()
    }

    /// Gets the configured token address.
    pub fn get_token(env: &Env) -> Address {
        env.storage().instance().get(&TOKEN_KEY).unwrap()
    }

    /// Gets the fixed withdrawal amount.
    pub fn get_fixed_amount(_env: &Env) -> i128 {
        FIXED_AMOUNT
    }

    /// Gets the verification key commitment for the withdrawal circuit.
    pub fn get_vk_commitment(env: &Env) -> BytesN<32> {
        let vk_bytes: Bytes = env.storage().instance().get(&VK_KEY).unwrap();
        vk_commitment(env, VK_COMMITMENT_DOMAIN, &vk_bytes)
    }

    /// Checks if a nullifier has been used.
    pub fn is_nullifier_used(env: &Env, nullifier: BytesN<32>) -> bool {
        let nullifiers: Vec<BytesN <<32>> =
            env.storage().instance().get(&NULL_KEY).unwrap_or(vec![&env]);
        nullifiers.contains(&nullifier)
    }
}
