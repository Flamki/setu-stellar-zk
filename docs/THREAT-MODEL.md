# Setu v1 Compact Threat Model

> **Prototype status:** Setu is an **unaudited** Stellar **testnet** prototype (`not production-secure`, not legal or compliance advice, with **stub** fiat/anchor corridors). Read alongside [Privacy & Compliance Limitations](./privacy-compliance-limitations.md), [Public Signal Serialization](./public-signal-serialization.md), and [Verification Key Commitments](./vk-commitments.md).

---

## 1. System Scope & Protected Invariants

Setu spans Circom circuits (`circuits/main.circom`, `circuits/disclosure.circom`), BLS12-381 Groth16 deserializers and verifiers (`libs/zk/`, `cli/circom2soroban`), the Soroban pool and disclosure contract (`contract/src/lib.rs`, `contract/src/disclosure.rs`), and the static web workspace (`site/`).

| Security / Privacy Invariant | Enforcing Mechanism | Prototype Status |
| --- | --- | --- |
| **Pool Solvency** | `withdraw` verifies a Groth16 proof over `(nullifierHash, withdrawnValue, stateRoot, associationRoot)` before transferring `FIXED_AMOUNT` | Enforced on-chain (`contract/src/lib.rs`), conditional on trusted-setup integrity |
| **Single-Spend Nullifiers** | `nullifiers.contains(&nullifier)` rejects duplicate spends before token transfer and before emitting `Withdrawal` | Enforced on-chain (`contract/src/lib.rs`, `#12`) |
| **Canonical Scalar Inputs** | `PublicSignals::from_bytes` rejects scalars `>= r` (`Groth16Error::NonCanonicalPublicSignal`) | Enforced in `libs/zk/src/lib.rs` and `cli/circom2soroban` (`#8`, `#9`, `#21`) |
| **Deposit-Withdrawal Unlinkability (Pre-Disclosure)** | Zero-knowledge Merkle membership in `stateRoot` hides which `commitment` leaf produced `nullifierHash` | Cryptographically sound in-circuit, but weakened on-chain without a relayer (`#18`) |
| **Selective-Disclosure Value Soundness** | `verify_disclosure` checks spent `nullifierHash`, pool leaf `commitment`, and Groth16 proof tying `discloseHash` `value` to leaf `value` | `value` is bound; `recipientId` and `purpose` remain **prover-asserted** (`#1`, `#30`) |
| **Verification-Key Transparency** | Domain-separated SHA-256 commitments (`get_vk_commitment`, `get_disclosure_vk_commitment`, `("dvk", "set")` event) | Enforced on-chain (`#5`, `#10`, `#19`) |

---

## 2. Adversary Model (In-Scope vs. Out-of-Scope)

### In-Scope Adversaries
1. **Malicious Withdrawer / Double-Spender:** Attempts to withdraw without a valid deposit, replay a spent proof, mutate public-signal byte encodings (`n + r`), or bypass association-set checks.
2. **Dishonest Disclosure Prover:** Owns (or fabricates) pool activity and attempts to convince an auditor of an unspent withdrawal, a non-existent deposit leaf, an inflated/deflated `value`, or fabricated `recipientId` / `purpose` metadata.
3. **Passive On-Chain Analyst / Curious Observer:** Monitors public Stellar ledger envelopes, Soroban contract storage/events, timing, and any on-chain `verify_disclosure` invocations to link depositors, withdrawers, and disclosed receipts.
4. **Compromised or Curious Relayer / Anchor (Boundary Adversary):** Inspects withdrawal submission metadata, censors relayed withdrawals, or misbinds off-ramp payout attestations once live corridors exist.
5. **Misconfigured or Compromised `admin` / Setup Operator:** Rotates `associationRoot` or `disclosure_vk` maliciously, or retains toxic waste from the single-contributor local Groth16 setup.

### Out-of-Scope Adversaries (v1 Prototype)
1. **Cryptographic Primitive / Host Breaks:** Adversaries breaking BLS12-381 pairing soundness, Poseidon255 collision resistance, SHA-256, or Soroban host `require_auth()` / WASM sandboxing.
2. **Client Endpoint Compromise:** Malware or browser extensions exfiltrating local note secrets `(nullifier, secret)`, `viewingKey`, or Freighter signing keys from the user's device.
3. **Network-Layer IP Surveillance:** Global passive adversaries correlating Horizon/Soroban RPC client IP addresses prior to transport-layer anonymization.
4. **Live Banking / Regulated Custody Settlement:** Real SEP-24/SEP-31 anchor settlement, chargebacks, and sanctions screening operations (currently UI stubs in `site/`).

---

## 3. Core Threat Surfaces, Mitigations & Linked Issues

### 3.1 Nullifier Replay & Pool State Integrity
- **Threat:** Replaying a valid withdrawal proof to drain pool funds, or generating a disclosure receipt for an unspent note or external commitment.
- **Current Mitigation:** `contract/src/lib.rs` checks `nullifiers.contains(&nullifier)` and records the nullifier prior to token transfer; `contract/src/disclosure.rs` (`verify_disclosure`) checks both `nullifiers.contains(&nullifier_hash)` and `leaves.contains(&commitment)` (`#64`).
- **Residual Risks & Issues:** Duplicate deposit commitments are not yet explicitly deduplicated (`#14`); `nullifiers` and `TREE_LEAVES_KEY` live in Soroban instance vectors with bounded scaling (`#15`, `#82`); string panic/error surfaces and remaining negative disclosure tests are tracked in `#20`, `#62`, `#63`, and `#65`.

### 3.2 Public Signal Malleability & Deserialization
- **Threat:** Submitting non-canonical 32-byte big-endian scalars (`s + k * r`) that reduce to `s mod r` inside BLS12-381 pairing checks while producing a distinct raw byte string in the on-chain nullifier set, enabling double-spends.
- **Current Mitigation:** `libs/zk/src/lib.rs` enforces canonical scalar bounds (`is_canonical_fr_bytes`, rejecting `>= r` and truncated buffers) before verification (`#8`, `#21`), and `cli/circom2soroban` validates curve membership, prime-order subgroups, and canonical scalars at conversion time (`#9`).
- **Residual Risks & Issues:** Property/fuzz testing over malformed proof and signal bytes (`#66`), golden `circom2soroban` snapshots (`#67`), and fixing `cli/coinutils` decimal truncation / empty-input handling (`#104`).

### 3.3 Trusted Setup & Verification-Key Governance
- **Threat:** Forging arbitrary withdrawal or disclosure proofs without a real deposit using Groth16 setup toxic waste, or an `admin` silently swapping the on-chain disclosure VK or bricking withdrawals via `set_association_root`.
- **Current Mitigation:** `get_vk_commitment()`, `get_disclosure_vk_commitment()`, and the `("dvk", "set")` event (`#5`, `#10`, `#19`) allow independent VK hash verification. `withdraw` requires `has_association_set() == true`, blocking the legacy `associationRoot == 0` conditional bypass still present in `circuits/main.circom`.
- **Residual Risks & Issues:** `scripts/build-circuits.sh` generates a **single-contributor local trusted setup (`not production-secure`)** that must be replaced by a multi-party ceremony (`#26`); `circuits/main.circom` should remove its `associationRoot == 0` branch (`#82`); association-root versioning (`#13`) and unauthorized `set_disclosure_vk` tests (`#61`) remain open.

### 3.4 Anchors & Prover-Asserted Remittance Metadata
- **Threat:** A dishonest prover withdraws funds for one counterparty/purpose and later generates a mathematically valid disclosure receipt claiming a benign `recipientId` and `purpose`, or users mistake mocked UI rails for live regulated settlement.
- **Current Mitigation:** `circuits/disclosure.circom` cryptographically binds `value` between `commitment` and `discloseHash` (`#24`), and `README.md` / `site/` document that fiat/INR anchor corridors are **stub** flows (`#56`).
- **Residual Risks & Issues:** `recipientId` and `purpose` are **prover-asserted** witness inputs not committed at deposit time — closing this requires binding them into the deposit commitment (`#1`), verifying anchor/off-ramp signatures in-circuit (`#30`, `#79`), adding value-tamper negative witness tests (`#23`), and surfacing live-vs-mocked badges across `site/` (`#41`, `#42`).

### 3.5 Relayers & Stellar Envelope Linkability
- **Threat:** Even with a zero-knowledge Merkle proof, submitting `withdraw` directly from a user-controlled Stellar account exposes the transaction source account, sequence number, and gas payer on-chain.
- **Current Mitigation:** Fixed-denomination pool transfers prevent trivial amount correlation, and the signer-funded relayer architecture and privacy tradeoffs are specified in [docs/relayer-withdrawal-design.md](./relayer-withdrawal-design.md) and [docs/relayer-withdrawal-privacy.md](./relayer-withdrawal-privacy.md) (`#18`).
- **Residual Risks & Issues:** **No relayer is implemented in v1**; envelope/gas privacy, relayer fee binding against griefing/front-running, and multi-denomination pool design remain open (`#17`, `#18`, `#29`, `#84`).

### 3.6 Auditor Privacy & Selective-Disclosure Linkability
- **Threat:** Third parties linking a user's deposit and withdrawal when disclosure verification runs publicly, or multiple receipts for the same withdrawal colliding on a deterministic `auditorTag`.
- **Current Mitigation:** Disclosure v2 derives `auditorTag = Poseidon255(viewingKey, nullifierHash, receiptNonce)` with explicit domain separation (`#3`, `#4`), so distinct `receiptNonce` values yield unlinkable tags to observers without `viewingKey`.
- **Residual Risks & Issues:** Calling `verify_disclosure` **on-chain** publishes both `nullifierHash` and `commitment` in ledger calldata, permanently linking that deposit and withdrawal publicly — bilateral audits should verify proofs off-chain via `snarkjs groth16 verify` and `circuits/auditor_recompute.circom`. In addition, `receiptNonce` uniqueness is prover-enforced rather than circuit-enforced (`#25`, `#44`, `#45`, `#79`).

---

## 4. Risk Register Summary

| ID | Threat / Gap | Severity (if shipped as-is) | Current Control | Tracking Issue(s) |
| --- | --- | --- | --- | --- |
| **R1** | Single-party local Groth16 trusted setup (`toxic waste` forgery) | Critical | Explicit testnet-only warning + VK commitments (`#5`, `#19`) | `#26`, `#79` |
| **R2** | `recipientId` and `purpose` are prover-asserted, not deposit/anchor-bound | High | Documented in limitations + witness tests (`#24`, `#56`) | `#1`, `#23`, `#30` |
| **R3** | No live relayer; `withdraw` envelope reveals Stellar fee-payer account | High | Design & privacy specs (`#18`); honest UI/README caveats | `#18`, `#84` |
| **R4** | On-chain `verify_disclosure` exposes `(nullifierHash, commitment)` link publicly | Medium | Off-chain `snarkjs` + `auditor_recompute.circom` path | `#25`, `#45`, `#79` |
| **R5** | Prover reuse of `receiptNonce` re-enables cross-receipt tag matching | Medium | v2 `receiptNonce` signal + `receipt_tags.json` vectors (`#3`) | `#79` |
| **R6** | Legacy `associationRoot == 0` bypass branch in `circuits/main.circom` | Medium | Blocked on-chain by `has_association_set()` in `withdraw` | `#13`, `#82` |
| **R7** | Unbounded instance-storage `Vec` growth & duplicate commitments | Medium | Fixed testnet usage; canonical byte checks (`#8`, `#9`) | `#14`, `#15`, `#66`, `#82` |
| **R8** | `cli/coinutils` decimal parser truncates fractional inputs / accepts empty string | Medium | CLI helper only (not on-chain verifier) | `#104` |
| **R9** | Anchor / off-ramp corridors in `site/` are non-custodial stubs | Low (Testnet) | Conservative claims enforced by `scripts/docs-check.cjs` | `#41`, `#42`, `#81` |
