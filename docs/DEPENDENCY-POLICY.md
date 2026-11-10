# Dependency Update & Version Pinning Policy

**Status: prototype.** Setu's cryptographic stack spans Circom circuits, `snarkjs` proving artifacts, Rust CLI converters (`cli/circom2soroban`, `cli/coinutils`), and the Soroban on-chain verifier (`libs/zk`, `contract`). Because a serialization change in any single layer breaks Groth16 verification on-chain, dependency updates follow strict compatibility rules.

## 1. Automated Updates (`.github/dependabot.yml`)

Dependabot checks three ecosystems weekly:
1. **Cargo (`/`)**: Patch and minor updates for workspace crates, with major version bumps ignored for `soroban-sdk`, `ark-bls12-381`, and `ark-serialize`.
2. **npm (`/site`)**: Updates for `site/` tooling (`site/package.json`).
3. **GitHub Actions (`/.github/workflows`)**: Workflow runner action updates (`actions/checkout`, `actions/setup-node`).

## 2. Cryptographic & Serialization Pinning Rules

| Dependency | Pinned Scope | Why It Is Pinned | Required Verification on Upgrade |
| --- | --- | --- | --- |
| `soroban-sdk` (`22.0.8`) | `Cargo.toml` workspace | Defines host BLS12-381 pairing (`G1Affine`, `G2Affine`, `Fr`) byte layout and Soroban storage/event APIs | `cargo test --workspace` + Soroban testnet WASM build |
| `ark-bls12-381` & `ark-serialize` (`0.5.0`) | `cli/circom2soroban`, `libs/zk`, `contract` | Used to serialize uncompressed G1 (96-byte) and G2 (192-byte) curve points consumed by `soroban-sdk` | `cargo test -p stellar-circom2soroban -p zk -p privacy-pools` |
| `circom` (`2.2.0`) & `snarkjs` (`0.7.x`) | Local proving toolchain (`scripts/check-prereqs.cjs`) | Emits decimal BLS12-381 field coordinates and public signal arrays | `node scripts/check-prereqs.cjs` and `libs/zk` public-signal fixture tests |

## 3. Merge Checklist for Dependency PRs

Before merging any dependency update PR:
1. Run `cargo test --workspace` and confirm all `libs/zk`, `cli/circom2soroban`, and `contract` tests pass.
2. Run `npm --prefix site run smoke` and `node scripts/docs-check.cjs`.
3. Confirm `get_vk_commitment` and `get_disclosure_vk_commitment` hashes in `contract` tests remain unchanged unless a deliberate circuit/VK migration is being performed.
