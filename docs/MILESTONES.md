# Setu v1 Hardening Milestone Plan

**Status: prototype.** Setu is currently a Stellar **testnet** prototype (`not audited`, `not production-secure`, with mock anchor corridors). This milestone plan organizes the open repository backlog into four sequential execution milestones required to harden Setu v1 without overstating current capabilities.

---

## Milestone 1: Cryptographic, Circuit & Verifier Hardening

**Objective:** Close circuit-level soundness gaps, pin cross-layer serialization invariants, and harden Groth16 byte deserialization.

| Track | Key Deliverables | Linked Issues | Exit Criteria |
| --- | --- | --- | --- |
| **Circuit Soundness** | Remove legacy `associationRoot == 0` bypass in `circuits/main.circom`; bind `recipientId` and `purpose` at deposit time or via signed off-ramp attestation | `#1`, `#2`, `#4` | Circuit rejects zero association root unconditionally; disclosed context is no longer purely prover-asserted |
| **Serializer & Signal Pinning** | Pin `withdrawal` (4 signals) and `disclosure` (5 signals with `receiptNonce`) wire order; add golden tests for `circom2soroban` and fuzz malformed proof/signal bytes | `#3`, `#21`, `#66`, `#67` | `cargo test --workspace` verifies golden fixtures and deterministic malformed-byte fuzzing without panics |
| **Contract Disclosure Guards** | Test unauthorized `set_disclosure_vk` calls, pre-VK `verify_disclosure`, post-withdrawal-only verification, and non-leaf commitment rejection | `#5`, `#61`, `#62`, `#63`, `#65` | 100% branch coverage on `contract/src/disclosure.rs` access control and state-membership gates |

---

## Milestone 2: Soroban Contract & Relayer Execution

**Objective:** Decouple withdrawal transaction submission from the depositor's Stellar account and bound on-chain storage growth.

| Track | Key Deliverables | Linked Issues | Exit Criteria |
| --- | --- | --- | --- |
| **Signer-Funded Relayer** | Implement relayer withdrawal submission and fee reimbursement so the withdrawing user does not sign the outer Stellar envelope | `#6`, `#16`–`#20` | Testnet withdrawal succeeds via relayer account; replay protection remains enforced by `nullifierHash` |
| **Storage & VK Commitments** | Bound nullifier/leaf storage growth with Soroban persistent/archival TTL management; pin `get_vk_commitment` and `get_disclosure_vk_commitment` in auditor tooling | `#5`, `#22`–`#30` | Auditors can verify installed withdrawal and disclosure VKs against repo SHA-256 commitments |

---

## Milestone 3: Client Proving, Wallet UX & Integration Boundaries

**Objective:** Harden the static operator/auditor workspace (`site/`) with automated route, clipboard, and Freighter fallback tests while keeping anchor stubs explicitly labeled.

| Track | Key Deliverables | Linked Issues | Exit Criteria |
| --- | --- | --- | --- |
| **Wallet & Receipt Safety** | Add automated tests for Freighter absent/rejected/wrong-network fallback states and receipt copy-to-clipboard witness exclusion | `#68`, `#69`, `#70` | `npm --prefix site run smoke` exercises route transitions, wallet fallbacks, and clipboard safety |
| **Anchor & Proving Boundaries** | Keep SEP-24/SEP-31 USDC/INR corridors clearly labeled as mock UI stubs until real testnet anchor sandboxes are wired | `#31`–`#50` | Zero unlabelled mock claims in UI or README; `node scripts/docs-check.cjs` passes |

---

## Milestone 4: Audit Readiness, Documentation & Release Governance

**Objective:** Give auditors, contributors, and judges complete technical references, threat models, and release gates.

| Track | Key Deliverables | Linked Issues | Exit Criteria |
| --- | --- | --- | --- |
| **Auditor & Security Docs** | Publish compact threat model (`docs/THREAT-MODEL.md`), known risks table (`docs/KNOWN-RISKS.md`), contract reference (`docs/CONTRACT-REFERENCE.md`), and auditor glossary (`docs/AUDITOR-GLOSSARY.md`) | `#7`, `#51`–`#58`, `#73` | Every contract entrypoint, public signal, and prototype risk is cross-referenced to source code and issues |
| **Release & Repo Governance** | Add release checklist (`docs/RELEASE-CHECKLIST.md`), artifact cleanup (`#78`), PR evidence template (`#72`), Dependabot policy (`#75`), and file attribution (`#76`) | `#59`, `#60`, `#72`, `#75`, `#76`, `#77`, `#78` | Every PR and release follows a repeatable verification and prototype-honesty checklist |
