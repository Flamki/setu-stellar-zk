# Differences from Upstream (`ymcrcat/soroban-privacy-pools`)

**Status: prototype.** Setu is forked from [`ymcrcat/soroban-privacy-pools`](https://github.com/ymcrcat/soroban-privacy-pools) (MIT License) and extends the base fixed-denomination Soroban privacy pool with **selective-disclosure compliance receipts**, **verification-key commitments**, **canonical scalar & curve-subgroup hardening**, and an **auditor/operator web workspace**.

---

## 1. Side-by-Side Architecture Comparison

| Layer | Upstream (`ymcrcat/soroban-privacy-pools`) | Setu (`Flamki/setu-stellar-zk`) |
| --- | --- | --- |
| **On-Chain Contract (`contract/`)** | Single `#contractimpl` in `contract/src/lib.rs` (`__constructor`, `deposit`, `withdraw`, `set_association_root`) | Adds `contract/src/disclosure.rs` (`set_disclosure_vk`, `has_disclosure_vk`, `verify_disclosure`, `get_disclosure_vk_commitment`), `get_vk_commitment` in `lib.rs`, mandatory non-zero association root before `withdraw`, and `withdraw` / `("dvk", "set")` events |
| **Zero-Knowledge Circuits (`circuits/`)** | `main.circom`, `commitment.circom`, `merkle.circom`, `poseidon255.circom` (withdrawal + association set membership only) | Adds `circuits/disclosure.circom` (5-signal v2 selective-disclosure receipt with `receiptNonce`) and `circuits/auditor_recompute.circom` (witness-only hash recomputation for auditors) |
| **Groth16 Verifier (`libs/zk/`)** | Deserializes `VerificationKey`, `Proof`, and `PublicSignals` without canonical scalar modulus checks | Enforces canonical BLS12-381 scalar range (`< r`, returning `Groth16Error::NonCanonicalPublicSignal`), exact wire length checks, and fixture-pinned public signal ordering tests (`libs/zk/testdata/`) |
| **Artifact Converter (`cli/circom2soroban/`)** | Unchecked curve-point deserialization and `unwrap()` error paths | Validates G1/G2 curve equation and prime-order subgroup membership, rejects non-canonical scalars (`>= r`) and `IC` length mismatches with typed `ConvertError` diagnostics |
| **Web Workspace (`site/`)** | None (CLI-only demo scripts) | Full static operator & auditor workspace (`site/index.html`, `site/app.js`, `site/server.cjs`, `site/smoke-check.cjs`) with Freighter testnet wallet integration and ZK flow simulator |
| **Documentation & Checks (`docs/`, `scripts/`)** | Basic README | Technical specs (`public-signal-serialization.md`, `vk-commitments.md`, `relayer-withdrawal-design.md`, `privacy-compliance-limitations.md`) enforced by `scripts/docs-check.cjs` |

---

## 2. Detailed Protocol Additions in Setu

### A. Selective-Disclosure Receipt Circuit (`circuits/disclosure.circom`)
Upstream proves only that a withdrawn coin belongs to an admin-approved association set (`associationRoot`). Setu adds a complementary post-withdrawal proof for auditors:
- Public signals: `[nullifierHash, commitment, discloseHash, auditorTag, receiptNonce]`.
- Proves that the withdrawal identified by `nullifierHash` spent the exact deposit leaf `commitment` whose committed value equals the disclosed `value` inside `discloseHash = Poseidon255(recipientId, purpose, value)`.
- Separates repeated disclosures of the same withdrawal using a public per-receipt `receiptNonce` in `auditorTag = Poseidon255(viewingKey, nullifierHash, receiptNonce)`.

### B. On-Chain Receipt Verifier (`contract/src/disclosure.rs`)
- `verify_disclosure(env, proof_bytes, pub_signals_bytes) -> bool` checks that:
  1. `nullifierHash` is already present in the contract's spent `null` storage list,
  2. `commitment` is a real deposited leaf in `TREE_LEAVES_KEY`, and
  3. The Groth16 proof verifies under the installed disclosure verification key (`dvk`).

### C. Verification-Key Commitments (`get_vk_commitment` & `get_disclosure_vk_commitment`)
- Exposes domain-separated SHA-256 commitments (`setu:vk-commitment:v1` and `setu:dvk-commitment:v1`) so auditors can confirm which withdrawal and disclosure circuits are installed on-chain without trusting contract storage.

---

## 3. Shared Prototype Limitations Inherited from Upstream

Setu remains honest about limitations inherited from the upstream prototype (see [docs/privacy-compliance-limitations.md](./privacy-compliance-limitations.md)):
- **Fixed 1-XLM denomination (`FIXED_AMOUNT`):** Both upstream and Setu transfer a fixed 1 XLM denomination on Stellar testnet; USDC/INR corridors in `site/` are UI stubs.
- **Single-contributor local trusted setup:** Setup scripts generate local `.ptau` and `.zkey` files (`not production-secure`).
- **Legacy `associationRoot == 0` branch in `circuits/main.circom`:** Retained in the circuit for backward compatibility with existing test vectors, but blocked on-chain in Setu's `withdraw` entrypoint via `has_association_set()`.
