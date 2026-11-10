# Setu Public Roadmap: Prototype Today vs Future Production Work

**Status: prototype.** This roadmap separates what is **live and verifiable on Stellar testnet today** from **design-only specifications** and **future engineering required before any production consideration**. See [docs/privacy-compliance-limitations.md](./privacy-compliance-limitations.md) for full caveats.

---

## 1. What Works Today (Testnet Prototype)

| Capability | Where It Lives | Current Guarantee |
| --- | --- | --- |
| **Fixed-Denomination Privacy Pool (1 XLM)** | `contract/src/lib.rs`, `circuits/main.circom`, `libs/lean-imt` | Authenticated deposits insert Poseidon255 commitments into a depth-20 LeanIMT; withdrawals verify a BLS12-381 Groth16 proof over `[nullifierHash, withdrawnValue, stateRoot, associationRoot]` and enforce non-zero `associationRoot` + nullifier uniqueness |
| **Selective-Disclosure Receipts (v2 Nonce-Separated)** | `contract/src/disclosure.rs`, `circuits/disclosure.circom`, `circuits/auditor_recompute.circom` | Proves a settled withdrawal (`nullifierHash` in pool) spent a real deposit leaf (`commitment` in pool) with committed `value`, bound to `discloseHash` and per-receipt `auditorTag = Poseidon255(viewingKey, nullifierHash, receiptNonce)` |
| **Verification-Key Commitments** | `contract/src/lib.rs`, `contract/src/disclosure.rs`, `docs/vk-commitments.md` | Domain-separated SHA-256 fingerprints (`get_vk_commitment`, `get_disclosure_vk_commitment`) and `("dvk", "set")` events so auditors can verify installed keys without trusting contract storage |
| **Canonical Signal & Curve Validation** | `libs/zk/src/lib.rs`, `cli/circom2soroban/src/main.rs` | Rejects non-canonical BLS12-381 scalars (`>= r`), off-curve/wrong-subgroup points, and mismatched signal counts |
| **Interactive Demo & Auditor Workspace** | `site/index.html`, `site/app.js`, `site/smoke-check.cjs` | Browser UI for Freighter testnet connection, ZK flow simulation, and disclosure receipt inspection |

---

## 2. Design-Only / Stubbed Today (Not Implemented)

| Area | Current State | Reference |
| --- | --- | --- |
| **Relayer Withdrawal Flow** | **Design specification only.** Withdrawals today are signed and submitted directly by the withdrawing account, exposing envelope metadata. | [docs/relayer-withdrawal-design.md](./relayer-withdrawal-design.md), [docs/relayer-withdrawal-privacy.md](./relayer-withdrawal-privacy.md) |
| **Fiat / Stablecoin Anchor Corridors (USDC -> INR)** | **UI stub only.** The pool transfers native testnet XLM; no SEP-24 or SEP-31 Stellar anchor, bank rail, or KYC provider is connected. | [docs/privacy-compliance-limitations.md](./privacy-compliance-limitations.md) |
| **Trusted Setup Ceremony** | **Local single-party setup (`not production-secure`).** Circuit keys are generated locally for testnet evaluation. | `setup.sh`, `run.sh` |

---

## 3. Future Hardening Phases

### Phase A — Circuit & Cryptographic Closure
- Bind `recipientId` and `purpose` into the deposit commitment or verify an anchor attestation signature inside `circuits/disclosure.circom`.
- Remove the legacy `associationRoot == 0` bypass branch in `circuits/main.circom` so the circuit enforces association membership independently of the contract check.
- Run a multi-party powers-of-tau and phase-2 Groth16 ceremony for both `main.circom` and `disclosure.circom`, pinning the resulting hashes in `docs/vk-commitments.md`.

### Phase B — Relayer & Storage Scalability
- Implement the signer-funded relayer withdrawal entrypoint and fee reimbursement logic on Soroban testnet.
- Migrate nullifier set and Merkle leaf storage from single instance vectors to Soroban persistent/archival storage entries with bounded TTL management.

### Phase C — Client Proving & Anchor Sandbox Integration
- Integrate in-browser WASM witness generation and Groth16 proving in `site/` with deterministic fixtures.
- Connect a Stellar testnet SEP-24/SEP-31 anchor sandbox for end-to-end testnet corridor demos.
- Commission an independent cryptography and smart-contract security audit prior to any non-testnet use.
