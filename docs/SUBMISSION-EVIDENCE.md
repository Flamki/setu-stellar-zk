# Setu Submission Evidence & Verification Index

> **Prototype notice:** Setu runs on **Stellar Testnet**, is **not audited**, uses a **local single-contributor Groth16 trusted setup** (`not production-secure`), and is **not legal, compliance, or financial advice**. For full cryptographic and regulatory caveats, read [Privacy & Compliance Limitations](./privacy-compliance-limitations.md).

This page gives reviewers a single scannable index separating **live testnet evidence**, **local proof & Rust workspace verification**, **UI screenshot & static bundle checks**, and **product mock boundaries**.

---

## 1. Live Stellar Testnet Evidence

All transactions below executed against the deployed `PrivacyPoolsContract` on Stellar Testnet:

- **Contract ID:** `CDXLQFYQJVDXBZDI5QVYRAM5TGPMZQWS424FCQWYVNGSKSSHPU6XXAXT` ([Stellar Lab](https://lab.stellar.org/r/testnet/contract/CDXLQFYQJVDXBZDI5QVYRAM5TGPMZQWS424FCQWYVNGSKSSHPU6XXAXT))
- **Stored Withdrawal Nullifier (on-chain):** `4acc5489ab80200caae1eca0b44dd2335e92931ff51e358e4fc7381378367816`

| Step | Stellar Expert Testnet Link | What It Verifies On-Chain |
| --- | --- | --- |
| **1. WASM Upload** | [`c9c8ed3e...73f701b51`](https://stellar.expert/explorer/testnet/tx/c9c8ed3e1f6b988b76b29526967195aec4b135dc4bc29ad6fabde0073f701b51) | Uploads compiled Soroban `privacy-pools` WASM (`contract/src/lib.rs` + `contract/src/disclosure.rs`) |
| **2. Contract Deploy** | [`3b9a86af...0abb019a`](https://stellar.expert/explorer/testnet/tx/3b9a86af7273750b54324c96fb3129a7677f59b7eb288b3c5de6dfe60abb019a) | Instantiates pool with withdrawal Groth16 VK, testnet XLM SAC token, admin, and depth-20 LeanIMT |
| **3. Pool Deposit (`deposit`)** | [`adf90170...9f9eb458`](https://stellar.expert/explorer/testnet/tx/adf901702fea3fcd09e0b8a94d19388d23d7579d05cbe8f7210b30489f9eb458) | Transfers fixed denomination (`1000000000` stroops) and inserts Poseidon255 commitment leaf |
| **4. Association Root (`set_association_root`)** | [`709cf0d7...faafa938`](https://stellar.expert/explorer/testnet/tx/709cf0d7530259e33b0af6ba0aa46d811947b1aa71a33f66042e275dfaafa938) | Admin sets non-zero `associationRoot` required before private withdrawals |
| **5. Private Withdrawal (`withdraw`)** | [`6d19b21f...463cfb9a`](https://stellar.expert/explorer/testnet/tx/6d19b21f5f96ac4237ce85882c81c9d991cd88cfb4859fd59c359b28463cfb9a) | Verifies 4-signal BLS12-381 Groth16 withdrawal proof on-chain, stores `nullifierHash`, and pays recipient |
| **6. Disclosure VK Install (`set_disclosure_vk`)** | [`d35c95b0...d64c6e0`](https://stellar.expert/explorer/testnet/tx/d35c95b0c65ddaedee2b85cd52a5d248571abb36d56c42c3599235ef8d64c6e0) | Installs selective-disclosure Groth16 VK (`dvk`) for on-chain receipt checks |

> **v1 vs. v2 circuit note:** The recorded testnet transactions above verified the initial **v1** 4-signal disclosure circuit prior to adding the per-receipt domain-separation nonce (`receiptNonce`). Current `main` uses the **v2** 5-signal disclosure circuit (`[nullifierHash, commitment, discloseHash, auditorTag, receiptNonce]`); re-running `bash scripts/build-circuits.sh` and `scripts/live_testnet_e2e.ps1` regenerates v2 keys and proofs.

---

## 2. Proof Results, Fixtures & UI Screenshot

- **On-chain & off-chain proof results (recorded E2E run):**
  - `snarkjs groth16 verify` on the disclosure proof returned `OK!`.
  - `verify_disclosure(validProof, validPublicSignals)` returned `true`.
  - `verify_disclosure(validProof, tamperedPublicSignals)` returned `false` (rejecting tampered `discloseHash` / `auditorTag`).
- **Pinned v2 public-signal & tag fixtures:**
  - Withdrawal & disclosure wire-order fixtures: [`libs/zk/testdata/public_signals/`](../libs/zk/testdata/public_signals) (verified by `cargo test -p zk`, documented in [`docs/public-signal-serialization.md`](./public-signal-serialization.md)).
  - v2 `auditorTag` nonce derivation vectors: [`libs/zk/testdata/receipt_tags.json`](../libs/zk/testdata/receipt_tags.json).
- **Product workspace screenshot:**
  - Operator & auditor web UI preview: [`site/assets/setu-product.png`](../site/assets/setu-product.png) (rendered by [`site/index.html`](../site/index.html)).

![Setu Operator and Auditor Workspace Screenshot](../site/assets/setu-product.png)

---

## 3. Fast Local Verification Commands

Reviewers can verify the Rust contracts, Groth16 deserializers, documentation honesty checks, circuit build script dry-run, and static site smoke suite locally:

```bash
# 1. Run Soroban contract, BLS12-381 Groth16 verifier, LeanIMT, and CLI converter tests
cargo test --workspace

# 2. Verify documentation completeness, tx-hash parity, and prototype-honesty guardrails
node scripts/docs-check.cjs

# 3. Validate circuit build pipeline flags without running a multi-minute Groth16 setup
bash scripts/build-circuits.sh --dry-run --all

# 4. Run static web bundle syntax, asset reference, and HTTP smoke checks
npm --prefix site run smoke
```

*(Optional environment check for full circuit recompilation with `circom` 2.2.x + `snarkjs` 0.7.x: `node scripts/check-prereqs.cjs`.)*

---

## 4. Claim-to-Code Map

| Claim / Capability | Implementation & Test Paths |
| --- | --- |
| **Pool deposit, Groth16 withdrawal, & VK commitment** | [`contract/src/lib.rs`](../contract/src/lib.rs), [`docs/vk-commitments.md`](./vk-commitments.md) |
| **Selective-disclosure on-chain verifier (`verify_disclosure`)** | [`contract/src/disclosure.rs`](../contract/src/disclosure.rs), [`contract/src/test.rs`](../contract/src/test.rs) |
| **Withdrawal & v2 selective-disclosure Circom circuits** | [`circuits/main.circom`](../circuits/main.circom), [`circuits/disclosure.circom`](../circuits/disclosure.circom), [`circuits/auditor_recompute.circom`](../circuits/auditor_recompute.circom) |
| **Canonical BLS12-381 scalar & proof deserialization** | [`libs/zk/src/lib.rs`](../libs/zk/src/lib.rs), [`libs/zk/src/test.rs`](../libs/zk/src/test.rs) |
| **snarkjs -> Soroban artifact converter & subgroup checks** | [`cli/circom2soroban/src/main.rs`](../cli/circom2soroban/src/main.rs) |
| **Reproducible testnet E2E script** | [`scripts/live_testnet_e2e.ps1`](../scripts/live_testnet_e2e.ps1) |
| **Static web workspace & smoke test** | [`site/index.html`](../site/index.html), [`site/app.js`](../site/app.js), [`site/smoke-check.cjs`](../site/smoke-check.cjs) |

---

## 5. Boundary Matrix: Live Testnet vs. Local Proof vs. Product Mock

| Surface | Evidence Tier | Honest Prototype Boundary |
| --- | --- | --- |
| **Pool deposit, Merkle root, nullifier spend, & Groth16 withdrawal** | **Live Testnet** | Deployed and executed on Stellar Testnet using native testnet XLM (`1000000000` stroops fixed denomination) |
| **Selective-disclosure `value` & `commitment` linkage** | **Live Testnet (v1) + Local Proof/Tests (v2)** | Proves disclosed `value` matches the deposited leaf; v2 adds `receiptNonce` domain separation in [`circuits/disclosure.circom`](../circuits/disclosure.circom) |
| **Canonical scalar validation & VK SHA-256 commitments** | **Local Proof & Unit Tests** | Enforced in `libs/zk` and `contract/` (`cargo test --workspace`); prevents non-canonical scalar (`n + r`) replay |
| **Disclosed `recipientId` & `purpose` fields** | **Prover-Asserted Context (Not Deposit-Bound)** | Hashed into `discloseHash` by the prover at receipt time; **not** committed at deposit time or signed by an off-ramp |
| **Groth16 Powers-of-Tau / zkey setup** | **Local Dev Setup Only** | Single-contributor local ceremony (`not production-secure`); must be replaced before any mainnet use |
| **USD/USDC on-ramp & INR off-ramp remittance corridors** | **Product Mock / UI Stub** | Storyboard UI in `site/`; no live SEP-24/SEP-31 anchor, KYC provider, or fiat settlement rail is connected |
| **Browser ZK proving in `site/`** | **Product Mock / UI Simulator** | Web workspace demonstrates flow and inspects receipts; real witness/proof generation runs via CLI (`scripts/build-circuits.sh`) |
| **Gas-private relayer withdrawal** | **Design Proposal Only** | Documented in [`docs/relayer-withdrawal-design.md`](./relayer-withdrawal-design.md); callers currently submit `withdraw` directly |
