# Release Checklist for Future Product & Testnet Builds

**Status: prototype.** Setu is a Stellar **testnet** prototype (`not audited`, `not production-secure`, with stubbed anchor corridors). Use this checklist before tagging any future testnet release, hackathon submission snapshot, or staging deployment so that build artifacts, circuit keys, contract WASM, and documentation stay in sync.

---

## 1. Pre-Flight Environment & Artifact Hygiene

- [ ] **Toolchain prerequisites pass:**
  ```bash
  node scripts/check-prereqs.cjs
  ```
- [ ] **No generated proving or local secret artifacts in tree:**
  - Verify `output/`, `circuits/build/`, `circuits/disclosure_build/`, `*.zkey`, `*.ptau`, `*.wtns`, `.env`, and `site/auth-config.js` are absent from `git status --ignored`.
- [ ] **Tracked fixtures intact:**
  - Confirm `libs/zk/testdata/public_signals/withdrawal.json`, `libs/zk/testdata/public_signals/disclosure.json`, and `libs/zk/testdata/receipt_tags.json` are clean and unmodified unless intentionally regenerated.

## 2. Circuit, Serializer & Verifier Parity

- [ ] **Public signal order matches across all layers:**
  - `circuits/main.circom`: `[nullifierHash, withdrawnValue, stateRoot, associationRoot]` (4 signals).
  - `circuits/disclosure.circom`: `[nullifierHash, commitment, discloseHash, auditorTag, receiptNonce]` (5 signals).
  - Matches [docs/public-signal-serialization.md](./public-signal-serialization.md) and `README.md`.
- [ ] **CLI converter & ZK library tests pass:**
  ```bash
  cargo test -p zk -p stellar-circom2soroban -p lean-imt -p stellar-coinutils
  ```

## 3. Soroban Contract Build & Verification-Key Commitments

- [ ] **Contract unit and integration tests pass:**
  ```bash
  cargo test -p privacy-pools
  ```
- [ ] **Soroban WASM compiles cleanly for target `wasm32v1-none` / `stellar contract build`:**
  ```bash
  stellar contract build
  ```
- [ ] **Verification-key commitments pinned:**
  - Record the deployed contract ID and verify `get_vk_commitment()` (`sha256("setu:vk-commitment:v1" || vk_bytes)`) and `get_disclosure_vk_commitment()` (`sha256("setu:dvk-commitment:v1" || dvk_bytes)`) match the expected hashes in [docs/vk-commitments.md](./vk-commitments.md).
  - Confirm `has_association_set()` and `has_disclosure_vk()` both return `true` on the target testnet deployment.

## 4. Site Workspace & Deployment Smoke Check

- [ ] **Site smoke check passes locally:**
  ```bash
  npm --prefix site run smoke
  ```
- [ ] **Auth fallback verified:**
  - When `SETU_SUPABASE_URL` / `SETU_SUPABASE_ANON_KEY` are unset, `node site/build-config.cjs` emits `enabled: false` and the login/signup controls render disabled with clear provisioning copy.

## 5. Documentation & Prototype Honesty Gate

- [ ] **Automated documentation & honesty check passes:**
  ```bash
  node scripts/docs-check.cjs
  ```
- [ ] **Conservative claims preserved:**
  - `README.md`, `site/index.html`, and `docs/privacy-compliance-limitations.md` explicitly state that Setu is an unaudited testnet prototype, the Groth16 setup is single-party (`not production-secure`), `recipientId`/`purpose` are prover-asserted in v1/v2, and USDC/INR anchor rails are illustrative stubs.
