# Setu Auditor Glossary

**Status: prototype.** This glossary defines the exact cryptographic signals, hashes, and Soroban contract concepts an auditor encounters when verifying a Setu withdrawal or selective-disclosure receipt. Read alongside [docs/privacy-compliance-limitations.md](./privacy-compliance-limitations.md) and [docs/public-signal-serialization.md](./public-signal-serialization.md).

---

## 1. Core Note & Pool Primitives

| Term | Formula / Type | Public or Private? | Definition & Auditor Relevance |
| --- | --- | --- | --- |
| **`value`** | BLS12-381 `Fr` scalar (`i128` stroops on-chain) | Private in note; disclosed in receipt | The committed note value. On the current Stellar testnet deployment, deposits and withdrawals use a fixed denomination (`FIXED_AMOUNT = 1_000_000_000` stroops / 1 XLM in test units). |
| **`label`** | BLS12-381 `Fr` scalar | Private | Identifier binding a deposit note to an association set leaf so the withdrawer can prove membership under `associationRoot`. |
| **`nullifier`** | BLS12-381 `Fr` scalar | **Strictly Private** | Secret randomness chosen at deposit time to derive `nullifierHash`. Never revealed on-chain or to the auditor. |
| **`secret`** | BLS12-381 `Fr` scalar | **Strictly Private** | Secret randomness chosen at deposit time to blind the commitment. Never revealed on-chain or to the auditor. |
| **`precommitment`** | `Poseidon255(nullifier, secret)` | Private intermediate | Intermediate hash combining the note's two secret scalars inside `circuits/commitment.circom` and `circuits/disclosure.circom`. |
| **`commitment`** | `Poseidon255(value, label, precommitment)` | Public on deposit (`BytesN<32>`); Public Signal `[1]` in disclosure | The 32-byte leaf inserted into the contract's Merkle tree on `deposit`. Unlinkable to `nullifierHash` during normal withdrawal; intentionally linked inside a selective-disclosure receipt. |
| **`nullifierHash`** | `Poseidon255(nullifier)` | Public Signal `[0]` in both `withdraw` and `verify_disclosure` | Deterministic spent marker stored in the contract's `null` list upon withdrawal to prevent double-spending and emitted in the `withdraw` event. |
| **`stateRoot`** | `BytesN<32>` / `Fr` (Public Signal `[2]` in `withdraw`) | Public | Root of the depth-20 Lean Incremental Merkle Tree (`libs/lean-imt`) holding all deposited `commitment` leaves. |
| **`associationRoot`** | `BytesN<32>` / `Fr` (Public Signal `[3]` in `withdraw`) | Public (admin-set) | Merkle root of approved deposit `label`s set by the contract admin via `set_association_root`. The contract requires a non-zero root before any withdrawal can execute. |

---

## 2. Selective-Disclosure Receipt Signals (`circuits/disclosure.circom`)

| Term | Formula / Wire Index | Public or Private? | Definition & Caveats |
| --- | --- | --- | --- |
| **`recipientId`** | BLS12-381 `Fr` scalar | Private circuit input (shared in cleartext with auditor) | Numeric/hashed identifier for the remittance beneficiary. **Prototype caveat:** In v1/v2, `recipientId` is prover-asserted at receipt generation time, not committed at deposit time. |
| **`purpose`** | BLS12-381 `Fr` scalar | Private circuit input (shared in cleartext with auditor) | Numeric/hashed remittance purpose code (e.g., family support). **Prototype caveat:** Prover-asserted in v1/v2, not committed at deposit time. |
| **`discloseHash`** | `Poseidon255(recipientId, purpose, value)` (Signal `[2]`) | Public Signal `[2]` in disclosure | Binds the cleartext `(recipientId, purpose, value)` tuple given to the auditor to the zero-knowledge proof. The auditor recomputes this hash via `circuits/auditor_recompute.circom`. |
| **`viewingKey`** | BLS12-381 `Fr` scalar | Shared secret between prover and auditor | Symmetric viewing key used to compute `auditorTag`. Does not prove the auditor's legal identity or regulatory status. |
| **`receiptNonce`** | BLS12-381 `Fr` scalar (Signal `[4]`) | Public Signal `[4]` in v2 disclosure | Per-receipt public nonce chosen by the prover so two disclosures for the same withdrawal (`nullifierHash`) under the same `viewingKey` produce distinct `auditorTag` values. **Caveat:** The circuit cannot force a prover to choose a fresh nonce. |
| **`auditorTag`** | `Poseidon255(viewingKey, nullifierHash, receiptNonce)` (Signal `[3]`) | Public Signal `[3]` in disclosure | Domain-separated tag allowing an auditor who holds `viewingKey` and reads the public `nullifierHash` + `receiptNonce` to confirm the receipt was addressed to them. (Archived v1 receipts omitted `receiptNonce`.) |

---

## 3. Verification-Key Commitments (`docs/vk-commitments.md`)

| Term | Formula | On-Chain Getter | Purpose |
| --- | --- | --- | --- |
| **Withdrawal VK Commitment** | `sha256("setu:vk-commitment:v1" \|\| vk_bytes)` | `get_vk_commitment()` | 32-byte fingerprint of the withdrawal Groth16 verification key installed by `__constructor`. |
| **Disclosure VK Commitment** | `sha256("setu:dvk-commitment:v1" \|\| dvk_bytes)` | `get_disclosure_vk_commitment()` | 32-byte fingerprint of the disclosure Groth16 verification key installed by `set_disclosure_vk` (also published in the `("dvk", "set")` event). |
