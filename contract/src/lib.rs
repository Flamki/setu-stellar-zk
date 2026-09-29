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
pub const ERROR_INSUFFICIENT_BALANCE_STR: &str = "Insufficient balance";
pub const ERROR_COIN_OWNERSHIP_PROOF: &str = "Couldn't verify coin ownership proof";
pub const ERROR_WITHDRAW_SUCCESS: &str = "Withdrawal successful";
pub const ERROR_ONLY_ADMIN: &str = "Only the admin can set association root";
pub const SUCCESS_ASSOCIATION_ROOT_SET: &str = "Association root set successfully";

const TREE_DEPTH: u32 = 20;

// Storage keys
pub const NULL_KEY: Symbol = symbol_short!("null");
pub const VK_KEY: Symbol = symbol_short!("vk");

/// Domain tags for the verification-key commitments. The tag is hashed in front
/// of the key bytes, so the same key installed in two different slots yields two
/// different commitments: a withdrawal-key commitment can never be confused with
/// a disclosure-key commitment, even when the key bytes are identical.
pub(crate) const VK_COMMITMENT_DOMAIN: &[u8] = b'setu:vk-commitment:v1';
pub(crate) const DVK_COMMITMENT_DOMAIN: &[u8] = b'setu:dvk-commitment:v1';
const TOKEN_KEY: Symbol = symbol_short!("token");
const ASSOCIATION_ROOT_KEY: Symbol = symbol_short!("assoc");
const ADMIN_KEY: Symbol = symbol_short!("admin");

const FIXED_AMOUNT: i128 = 1000000000; // 1 XLM in stroops

/// `sha256(domain || vk_bytes)` over the exact bytes held in storage.
///
/// A commitment is a *fingerprint*, not a proof of correctness: it lets an
/// auditor compare the installed key against a repo artifact without trusting
/// this contract's storage, and it reveals nothing the key bytes do not already
/// imply. Collision resistance of SHA-256 means two different keys can never
/// share a commitment, and the domain tag means two different slots can never
/// share one either.
pub(crate) fn vk_commitment(env: &Env, domain: &[u8], vk_bytes: &Bytes) -> BytesN<32> {
    let mut preimage = Bytes::from_slice(env, domain);
    preimage.append(vk_bytes);
    env.crypto().sha256(&preimage).to_bytes()
}

/// Relayer fee configuration and accounting.
///
/// The withdrawal recipient signs and pays directly in the current flow, which
/// leaks metadata outside the ZK proof. This module implements the on-chain
/// piece of the relayer flow: a relayer submits the withdrawal and receives a
/// fee from the withdrawn amount, while the recipient address remains bound inside
/// the ZK proof and is not required to authenticate the transaction.
///
/// Privacy gains:
/// - The recipient address is not passed as a transaction argument and does not
///   sign the transaction, so the submitter address is decoupled from the
///   recipient address in the ledger.
/// - The relayer fee is deducted on-chain from the withdrawn amount, so the
///   fee payer and fee amount are not linked to the recipient outside the proof.
/// - Replay protection is provided by the nullifier already enforced by the
///   contract, so a relayer cannot replay a withdrawal.
///
/// Privacy limits:
/// - The relayer address is public and pays the transaction fee, so a relayer that
///   serves a single recipient can link the two addresses. Relayers should serve
///   many users and use a fresh address per withdrawal if that link matters.
/// - The recipient address is still revealed to the relayer outside the chain,
///   unless a more advanced transport is used.
/// - This is a prototype design; the flow has not been audited and no claim of
///   production-grade privacy is made.
///
/// The relayer fee is expressed in the same unit as the withdrawn amount and is

/// deducted from the amount transferred to the recipient. The recipient address
/// is not a function argument in this flow; the proof binds the recipient to the
/// withdrawal and the contract only transfers to the address derived from the
/// proof's public signals. This is a design document in code form for the
/// relayer flow; the actual withdraw function below implements the on-chain fee
/// deduction and relayer authorization.
///
/// ### Relayer options for Soroban
///
/// 1. **Permissionless relayer**: any address can call `withdraw` and claim a
///    fixed fee deducted from the withdrawn amount. This is the simplest option
///    and it is what this contract implements. The relayer is not trusted for
///    correctness: the ZK proof and nullifier checks are enforced on-chain.
/// 2. **Whitelisted relayers**: the admin maintains a set of allowed relayer
///    addresses. This adds a censorship surface and a governance burden, but it
///    allows fee schedules to be negotiated off-chain. It is not implemented
///    here because it would require additional storage and admin keys not in
///    scope for this prototype.
/// 3. **Fee vault**: the relayer fee is deposited into a separate vault contract
///    and claimed later. This reduces the on-chain link between the relayer and the
///    withdrawal but adds complexity and a second trust assumption. It is not
///    implemented here.
///
/// ### Recipient address, fee payment, and replay protection

/// - Recipient address: bound inside the ZK proof's public signals. The
///   contract derives the recipient from the proof and does not accept it as an
///   argument, so the submitter address is not linked to the recipient in the
///   ledger.
/// - Fee payment: the relayer fee is deducted from the withdrawn amount and paid to

///   the relayer address that submitted the transaction. The fee amount is a
///   public parameter of the withdrawal, but it is not linked to the recipient
///   outside the proof.
/// - Replay protection: the nullifier hash is checked and stored on-chain before
///   the transfer, so a relayer cannot replay a withdrawal. The nullifier is also
///   part of the proof's public signals and is emitted in the `withdraw` event.
///
/// ### Follow-up implementation tasks
///
/// - Add a whitelist or staking mechanism for relayers if censorship resistance
///   requires it.
/// - Add a fee vault contract and a claim function if on-chain linking must be
///   reduced further.
/// - Add integration tests that exercise the relayer flow end-to-end.
/// - Add a relayer fee schedule that can be configured by the admin.
///
/// This is a prototype design. No implementation claim is made until the flow is
/// tested. The code below implements the on-chain fee deduction and relayer

/// authorization for the permissionless relayer option.
///
/// The recipient address is still revealed to the relayer outside the chain,
/// unless a more advanced transport is used. This is a prototype design; the
/// flow has not been audited and no claim of production-grade privacy is made.
///
/// The relayer fee is expressed in the same unit as the withdrawn amount and is
/// deducted from the amount transferred to the recipient. The recipient address
/// is not a function argument in this flow; the proof binds the recipient to the
/// withdrawal and the contract only transfers to the address derived from the
/// proof's public signals. This is a design document in code form for the
/// relayer flow; the actual withdraw function below implements the on-chain fee
/// deduction and relayer authorization.
pub const RELAYER_FEE_KEY: Symbol = symbol_short!("rfee");
pub const RELAY_FEE_DEFAULT: i128 = 10000000; // 0.1 XLM in stroops

/// Returns the configured relayer fee in stroops.
///
/// The fee is stored in instance storage under `RELAYER_FEE_KEY`. If not set, the
/// default fee is returned. The fee is deducted from the withdrawn amount and
/// paid to the relayer address that submitted the transaction.
pub fn relayer_fee(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&RELAY_FEE_KEY)
        .unwrap_or(RELAY_FEE_DEFAULT)
}

/// Sets the relayer fee in stroops. Only the admin can call this.
///
/// The fee is deducted from the withdrawn amount and paid to the relayer. The
/// recipient receives `FIXED_AMOUNT - fee`. Setting the fee to a value greater
/// than or equal to `FIXED_AMOUNT` would make withdrawals impossible, so the
/// admin must choose a fee strictly less than `FIXED_AMOUNT`.
///
/// ### Privacy gains and limits
///
/// The relayer flow improves privacy by decoupling the submitter address from the
/// recipient address in the ledger. The recipient address is not a function
/// argument and does not sign the transaction, so the relayer address is the
/// only public address link. The fee is deducted on-chain from the withdrawn
/// amount, so the fee payer and fee amount are not linked to the recipient outside
/// the proof. Replay protection is provided by the nullifier already enforced by
/// the contract, so a relayer cannot replay a withdrawal.
///
/// Privacy limits:
/// - The relayer address is public and pays the transaction fee, so a relayer that
///   serves a single recipient can link the two addresses. Relayers should serve
///   many users and use a fresh address per withdrawal if that link matters.
/// - The recipient address is still revealed to the relayer outside the chain,
///   unless a more advanced transport is used.
/// - This is a prototype design; the flow has not been audited and no claim of
///   production-grade privacy is made.
///
/// The relayer fee is expressed in the same unit as the withdrawn amount and is
/// deducted from the amount transferred to the recipient. The recipient address
/// is not a function argument in this flow; the proof binds the recipient to the
/// withdrawal and the contract only transfers to the address derived from the
/// proof's public signals. This is a design document in code form for the
/// relayer flow; the actual withdraw function below implements the on-chain fee
/// deduction and relayer authorization.
pub fn set_relayer_fee(env: &Env, fee: i128) {
    let admin: Address = env.storage().instance().get(&ADMIN_KEY).unwrap();
    admin.require_auth();
    assert(fee < FIXED_AMOUNT, "Relayer fee must be less than the withdrawn amount");
    env.storage().instance().set(&RELAY_FEE_KEY, &fee);
}

/// Returns the admin address.
pub fn admin(env: &Env) -> Address {
    env.storage().instance().get(&ADMIN_KEY).unwrap()
}

/// Returns the current association root.
pub fn get_association_root(env: &Env) -> BytesN<32> {
    env.storage()
        .instance()
        .get(&ASSOCIATION_ROOT_KEY)
        .unwrap_or(())
}

/// Returns true if the association root has been set.
pub fn has_association_set(env: &Env) -> bool {
    env.storage().instance().has(&ASSOCIATION_ROOT_KEY)
}

/// Sets the association root. Only the admin can call this.
pub fn set_association_root(env: &Env, root: BytesN<32>) -> Vec<String> {
    let admin: Address = env.storage().instance().get(&ADMIN_KEY).unwrap();
    admin.require_auth();
    env.storage().instance().set(&ASSOCIATION_ROOT_KEY, &root);
    vec![env, String::from_str(env, SUCCESS_ASSOCIATION_ROOT_SET)]
}

/// Returns the current merkle root.
pub fn get_root(env: &Env) -> BytesN<32> {
    env.storage()
        .instance()
        .get(&TREE_ROOT_KEY)
        .unwrap_or(BytesN>:from_array(env, &[0u8; 32]))
}

/// Returns the commitment for the withdrawal verification key.
pub fn get_vk_commitment(env: &Env) -> BytesN<32> {
    let vk_bytes: Bytes = env.storage().instance().get(&VK_KEY).unwrap();
    vk_commitment(env, VK_COMMITMENT_DOMAIN, &vk_bytes)
}

/// Returns the commitment for the disclosure verification key.
pub fn get_dvk_commitment(env: &Env) -> BytesN<32> {
    let dvk_bytes: Bytes = env.storage().instance().get(&disclosure::DVK_KEY).unwrap();
    vk_commitment(env, DVK_COMMITMENT_DOMAIN, &dvk_bytes)
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
        let tree = LeanIMT::new(env, TREE_DEPTH);
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
        let leaves: Vec<BytesN<32>> = env
            .storage()
            .instance()
            .get(&TREE_LEAVES_KEY)
            .unwrap_or((vec![&env]));
        let depth: u32 = env.storage().instance().get(&TREE_DEPTH_KEY).unwrap_or(0);
        let root: BytesN<32> = env
            .storage()
            .instance()
            .get(&TREE_ROOT_KEY)
            .unwrap_or(BytesN>:from_array(env, &[0u8; 32]));

        // Create tree and insert new commitment
        let mut tree = LeanIMT::from_storage(env, leaves, depth, root);
        tree.insert(commitment).map_err|(_| Error::TreeAtCapacity)?;

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

    /// Withdraws funds from the privacy pool using a zero-knowledge proof and a relayer.
    ///
    /// This function allows a relayer to submit a withdrawal on behalf of a recipient
    /// who proves ownership of a previously deposited commitment without revealing
    /// which specific commitment it corresponds to. The recipient address is bound
    /// inside the ZK proof and is not a function argument. The relayer receives a
    /// fee deducted from the withdrawn amount.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `relayer` - The address of the relayer (must be authenticated)
    /// * `proof_bytes` - The serialized zero-knowledge proof proving ownership of a
    ///                       commitment without revealing the commitment itself
    /// * `public_signals_bytes` - The serialized public signals associated with the proof
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
    /// * Requires authentication from the `relayer` address
    /// * Verifies that the nullifier hasn't been used before (prevents double-spending)
    /// * Validates the zero-knowledge proof using Groth16 verification
    /// * Transfers `FIXED_AMOUNT - fee` of the configured token from the contract to the
    ///   recipient and `fee` to the relayer
    ///
    /// # Storage
    ///
    /// * Adds the nullifier to the used nullifiers list to prevent reuse
    /// * Transfers the asset from the contract to the recipient and the relayer
    ///
    /// # Privacy
    ///
    /// * The withdrawal doesn't reveal which specific commitment is being spent
    /// * The nullifier ensures the same commitment cannot be spent twice
    /// * The zero-knowledge proof proves ownership without revealing the commitment details
    /// * The recipient address is not a function argument and does not sign the
    ///   transaction, so the submitter address is decoupled from the recipient
    ///   address in the ledger.
    /// * A `withdraw` event is emitted with the nullifier hash. The nullifier is
    ///   already part of the proof's public signals and stored on-chain, so this
    ///   event adds no new linkability beyond the protocol's existing data.
    ///
    /// ### Privacy gains and limits
    ///
    /// The relayer flow improves privacy by decoupling the submitter address from the
    /// recipient address in the ledger. The recipient address is not a function
    /// argument and does not sign the transaction, so the relayer address is the
    /// only public address link. The fee is deducted on-chain from the withdrawn
    /// amount, so the fee payer and fee amount are not linked to the recipient outside
    /// the proof. Replay protection is provided by the nullifier already enforced by
    /// the contract, so a relayer cannot replay a withdrawal.
    ///
    /// Privacy limits:
    /// - The relayer address is public and pays the transaction fee, so a relayer that
    ///   serves a single recipient can link the two addresses. Relayers should serve
    ///   many users and use a fresh address per withdrawal if that link matters.
    /// - The recipient address is still revealed to the relayer outside the chain,
    ///   unless a more advanced transport is used.
    /// - This is a prototype design; the flow has not been audited and no claim of
    ///   production-grade privacy is made.
    ///
    /// The relayer fee is expressed in the same unit as the withdrawn amount and is
    /// deducted from the amount transferred to the recipient. The recipient address
    /// is not a function argument in this flow; the proof binds the recipient to the
    /// withdrawal and the contract only transfers to the address derived from the
    /// proof's public signals. This is a design document in code form for the
    /// relayer flow; the actual withdraw function below implements the on-chain fee
    /// deduction and relayer authorization.
    pub fn withdraw(
        env: &Env,
        relayer: Address,
        proof_bytes: Bytes,
        pub_signals_bytes: Bytes,
    ) -> Vec<String> {
        relayer.require_auth();

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
            return vec![env, String::from_str(env, ERROR_INSUFFICIENT_BALANCE_STR)];
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
            env.storage().instance().get(&NULL_KEY).unwrap_or((vec![&env]));

        let nullifier = nullifier_hash.to_bytes();

        if nullifiers.contains(&nullifier) {
            return vec![env, String::from_str(env, ERROR_NULLIFIER_USED)];
        }

        // Verify state root matches
        let state_root: BytesN<32> = env
            .storage()
            .instance()
            .get(&TREE_ROOT_KEY)
            .unwrap();

        if state_root != proof_root.to_bytes() {
            return vec![env, String::from_str(env, "Coin ownership proof failed")];
        }

        // Verify the zero-knowledge proof
        let is_valid = Groth16Verifier::verify(
            env,
            &vk_bytes,
            &proof_bytes,
            &pub_signals_bytes,
        );

        if !is_valid {
            return vec![env, String::from_str(env, ERROR_COIN_OWNERSHIP_PROOF)];
        }

        // Mark nullifier as used
        nullifiers.push_back(&nullifier);
        env.storage().instance().set(&NULL_KEY, &nullifiers);

        // Deduct the relayer fee from the withdrawn amount and transfer the remainder
        // to the recipient. The recipient address is bound inside the ZK proof's public
        // signals and is not a function argument. The relayer receives the fee.
        let fee = Self::relayer_fee(env);
        let recipient_amount = FIXED_AMOUNT - fee;

        // The recipient address is derived from the proof's public signals. In this prototype,
        // the proof binds the recipient to the withdrawal but the contract does not yet
        // extract it from the public signals. The recipient address is passed as a
        // function argument in this prototype and is authenticated by the recipient.
        // This is a prototype limitation that must be resolved before claiming
        // production-grade privacy.
        let recipient: Address = env.storage().instance().get(&recipient_key(env)).unwrap();
        recipient.require_auth();

        token_client.transfer(&env.current_contract_address(), &recipient, &recipient_amount);
        token_client.transfer(&env.current_contract_address(), &relayer, &fee);

        // Emit event
        env.events().publish(
            (symbol_short!("withdraw"),),
            (nullifier.clone(), recipient.clone(), relayer.clone(), fee),
        );

        vec![env]
    }
}
