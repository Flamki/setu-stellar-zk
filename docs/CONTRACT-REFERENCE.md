# Soroban Contract Method Reference (`PrivacyPoolsContract`)

**Status: prototype.** This reference documents every public method exposed by `PrivacyPoolsContract` across [`contract/src/lib.rs`](../contract/src/lib.rs) and [`contract/src/disclosure.rs`](../contract/src/disclosure.rs). All state currently lives in Soroban `instance()` storage on the Stellar **testnet** (`not audited`, `not production-secure`). See [docs/privacy-compliance-limitations.md](./privacy-compliance-limitations.md).

---

## Summary Table

| Method | Source File | Auth Required | Mutates State? | Emits Event? |
| --- | --- | --- | --- | --- |
| `__constructor` | `contract/src/lib.rs` | Deployer (implicit) | Yes (init) | No |
| `deposit` | `contract/src/lib.rs` | `from.require_auth()` | Yes | No |
| `withdraw` | `contract/src/lib.rs` | `to.require_auth()` | Yes | `("withdraw",)` -> `nullifier` |
| `set_association_root` | `contract/src/lib.rs` | `caller.require_auth()` + `is_admin` | Yes | No |
| `get_association_root` | `contract/src/lib.rs` | None | No | No |
| `has_association_set` | `contract/src/lib.rs` | None | No | No |
| `get_merkle_root` | `contract/src/lib.rs` | None | No | No |
| `get_merkle_depth` | `contract/src/lib.rs` | None | No | No |
| `get_commitment_count` | `contract/src/lib.rs` | None | No | No |
| `get_commitments` | `contract/src/lib.rs` | None | No | No |
| `get_nullifiers` | `contract/src/lib.rs` | None | No | No |
| `get_balance` | `contract/src/lib.rs` | None | No | No |
| `get_admin` | `contract/src/lib.rs` | None | No | No |
| `get_vk_commitment` | `contract/src/lib.rs` | None | No | No |
| `set_disclosure_vk` | `contract/src/disclosure.rs` | `caller.require_auth()` + `caller == admin` | Yes | `("dvk", "set")` -> `commitment` |
| `get_disclosure_vk_commitment` | `contract/src/disclosure.rs` | None | No | No |
| `has_disclosure_vk` | `contract/src/disclosure.rs` | None | No | No |
| `verify_disclosure` | `contract/src/disclosure.rs` | None | No (read-only check; emits event on `true`) | `("withdraw",)` -> `nullifier_hash` |

---

## 1. Core Pool Entrypoints (`contract/src/lib.rs`)

### `__constructor(env: &Env, vk_bytes: Bytes, token_address: Address, admin: Address)`
- **Purpose:** Initializes the privacy pool contract on deployment.
- **Storage Writes:**
  - `admin` (`ADMIN_KEY`): Sets the contract administrator address.
  - `vk` (`VK_KEY`): Stores the serialized BLS12-381 Groth16 withdrawal verification key (`ic.len() == 5` for 4 public signals).
  - `token` (`TOKEN_KEY`): Stores the SAC token address (testnet XLM).
  - `TREE_LEAVES_KEY`, `TREE_DEPTH_KEY` (`20`), `TREE_ROOT_KEY`: Initializes an empty depth-20 `LeanIMT`.

### `deposit(env: &Env, from: Address, commitment: BytesN<32>) -> Result<u32, Error>`
- **Auth:** `from.require_auth()`.
- **Behavior:** Transfers `FIXED_AMOUNT` (`1_000_000_000` stroops) of the configured token from `from` to the contract, inserts `commitment` into the `LeanIMT`, updates `TREE_LEAVES_KEY`, `TREE_DEPTH_KEY`, and `TREE_ROOT_KEY`, and returns the 0-based `leaf_index`.
- **Errors:** Returns `Err(Error::TreeAtCapacity)` (`5`) if the depth-20 Merkle tree is full; panics if token transfer fails.

### `withdraw(env: &Env, to: Address, proof_bytes: Bytes, pub_signals_bytes: Bytes) -> Vec<String>`
- **Auth:** `to.require_auth()`.
- **Public Signals Order:** `[nullifierHash, withdrawnValue, stateRoot, associationRoot]` (must be exactly 4 canonical `Fr` scalars).
- **Preconditions & Checks:**
  1. Panics with `"Association root must be set before withdrawal"` if `has_association_set(env)` is `false`.
  2. Returns `["Insufficient balance"]` if contract token balance `< FIXED_AMOUNT`.
  3. Deserializes `Proof` and `PublicSignals` (`libs/zk`), returning `["Couldn't verify coin ownership proof"]` on any wire or non-canonical scalar error.
  4. Returns `["Association set root mismatch"]` if signal `[3]` (`associationRoot`) does not equal `get_association_root(env)`.
  5. Returns `["Nullifier already used"]` if signal `[0]` (`nullifierHash`) is already in `NULL_KEY`.
  6. Returns `["Couldn't verify coin ownership proof"]` if signal `[2]` (`stateRoot`) does not equal `get_merkle_root(env)` or if `Groth16Verifier::verify_proof` fails.
- **Effects on Success:** Appends `nullifierHash` to `NULL_KEY`, transfers `FIXED_AMOUNT` to `to`, publishes `("withdraw",)` event with `nullifierHash`, and returns an empty `Vec<String>` (`[]`).

### `set_association_root(env: &Env, caller: Address, association_root: BytesN<32>) -> Vec<String>`
- **Auth:** `caller.require_auth()`.
- **Behavior:** If `caller == get_admin(env)`, writes `association_root` to `ASSOCIATION_ROOT_KEY` and returns `["Association root set successfully"]`. Otherwise leaves storage unchanged and returns `["Only the admin can set association root"]`.

### Pool Read-Only Getters (`contract/src/lib.rs`)
- `get_association_root(env: &Env) -> BytesN<32>`: Returns the stored association root or `[0u8; 32]` if unset.
- `has_association_set(env: &Env) -> bool`: Returns `true` iff `get_association_root(env) != [0u8; 32]`.
- `get_merkle_root(env: &Env) -> BytesN<32>`: Returns the current `LeanIMT` root (`TREE_ROOT_KEY`).
- `get_merkle_depth(env: &Env) -> u32`: Returns the configured tree depth (`20`).
- `get_commitment_count(env: &Env) -> u32`: Returns the number of deposited leaves.
- `get_commitments(env: &Env) -> Vec<BytesN<32>>`: Returns all deposited leaf commitments in insertion order.
- `get_nullifiers(env: &Env) -> Vec<BytesN<32>>`: Returns all spent `nullifierHash` values.
- `get_balance(env: &Env) -> i128`: Queries the configured token contract for the pool's current balance.
- `get_admin(env: &Env) -> Address`: Returns the contract administrator address.
- `get_vk_commitment(env: &Env) -> Option<BytesN<32>>`: Returns `sha256("setu:vk-commitment:v1" || vk_bytes)` over the stored withdrawal verification key.

---

## 2. Selective-Disclosure Entrypoints (`contract/src/disclosure.rs`)

### `set_disclosure_vk(env: &Env, caller: Address, dvk_bytes: Bytes)`
- **Auth:** `caller.require_auth()`; panics with `"only admin can set disclosure vk"` if `caller != get_admin(env)`.
- **Effects:** Stores `dvk_bytes` under `DVK_KEY` (`"dvk"`) and publishes a `("dvk", "set")` event with `sha256("setu:dvk-commitment:v1" || dvk_bytes)`.

### `get_disclosure_vk_commitment(env: &Env) -> Option<BytesN<32>>`
- **Behavior:** Returns `Some(sha256("setu:dvk-commitment:v1" || dvk_bytes))` if a disclosure key is installed, or `None` prior to `set_disclosure_vk`.

### `has_disclosure_vk(env: &Env) -> bool`
- **Behavior:** Returns `true` if `DVK_KEY` is present in instance storage, `false` otherwise.

### `verify_disclosure(env: &Env, proof_bytes: Bytes, pub_signals_bytes: Bytes) -> bool`
- **Auth:** None (public verification entrypoint).
- **Public Signals Order:**
  - v2 (5 signals): `[nullifierHash, commitment, discloseHash, auditorTag, receiptNonce]`
  - v1 (4 signals, archived): `[nullifierHash, commitment, discloseHash, auditorTag]`
- **Verification Steps:**
  1. Returns `false` if `DVK_KEY` is not installed or fails `VerificationKey::from_bytes`.
  2. Returns `false` if `proof_bytes` or `pub_signals_bytes` fail deserialization (including non-canonical scalars `>= r`) or if `pub_signals.len()` is not `4` or `5`.
  3. Checks `get_nullifiers(env).contains(&nullifierHash)` (withdrawal must already be settled in this pool).
  4. Checks `get_commitments(env).contains(&commitment)` (commitment must be an actual deposit leaf in this pool).
  5. Runs `Groth16Verifier::verify_proof(env, vk, proof, &pub_signals)` (which also enforces `pub_signals.len() + 1 == vk.ic.len()`).
  6. If and only if verified, emits `("withdraw",)` with `nullifierHash` and returns `true`.
- **Privacy Warning:** Invoking `verify_disclosure` on-chain publishes the `(nullifierHash, commitment)` pair in transaction parameters. For single-auditor disclosure, verify the proof off-chain instead.
