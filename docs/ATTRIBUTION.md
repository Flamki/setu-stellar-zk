# Source Attribution & License Notes

**License:** MIT (`SPDX-License-Identifier: MIT`).

Setu is built on top of [`ymcrcat/soroban-privacy-pools`](https://github.com/ymcrcat/soroban-privacy-pools) (MIT License) and adds selective-disclosure receipt circuits, on-chain receipt verification, verification-key commitments, and an interactive testnet operator/auditor workspace.

## 1. Upstream Components (`ymcrcat/soroban-privacy-pools`)

The following components originate from the upstream Soroban Privacy Pools prototype and are retained under the MIT License:

| Path | Upstream Role | Setu Modifications |
| --- | --- | --- |
| `contract/src/lib.rs` | Core 1-XLM fixed-denomination privacy pool (`__constructor`, `deposit`, `withdraw`, `set_association_root`) | Added `withdraw` event emission, non-zero association root enforcement, and `get_vk_commitment` |
| `circuits/main.circom`, `circuits/commitment.circom`, `circuits/merkle.circom`, `circuits/poseidon255.circom` | BLS12-381 Poseidon255 commitment, LeanIMT Merkle proof, and withdrawal circuit | Preserved upstream circuit structure; documented `associationRoot == 0` legacy branch |
| `libs/lean-imt/` | Soroban Lean Incremental Merkle Tree library | Used unchanged for pool commitment storage |
| `libs/zk/` | Soroban BLS12-381 Groth16 verifier and wire types | Hardened with canonical scalar checks (`NonCanonicalPublicSignal`) and signal-order fixture tests |
| `cli/coinutils/`, `cli/circom2soroban/` | CLI tools for coin commitment generation and snarkjs-to-Soroban byte conversion | Added curve-point subgroup validation, canonical scalar checks, and unit/golden tests |

## 2. Setu-Original Components

The following files were authored for Setu and carry `SPDX-License-Identifier: MIT` attribution headers:

| Path | Purpose | Prototype Caveat |
| --- | --- | --- |
| `contract/src/disclosure.rs` | On-chain selective-disclosure receipt verifier (`set_disclosure_vk`, `verify_disclosure`, `get_disclosure_vk_commitment`) | Verifies `nullifierHash` spent status, `commitment` leaf membership, and Groth16 receipt proof; on-chain calls publish the `nullifierHash <-> commitment` link |
| `circuits/disclosure.circom` | 5-signal v2 selective-disclosure circuit (`[nullifierHash, commitment, discloseHash, auditorTag, receiptNonce]`) | `recipientId` and `purpose` are prover-asserted context in v1/v2, not committed at deposit time |
| `circuits/auditor_recompute.circom` | Off-chain auditor witness helper to recompute `discloseHash` and `auditorTag` | Does not authenticate auditor identity or force `receiptNonce` freshness |
| `site/` (`site/index.html`, `site/app.js`, `site/styles.css`, `site/server.cjs`, `site/smoke-check.cjs`) | Static testnet remittance desk, ZK simulator, and auditor receipt workspace | Fiat/USDC on-ramp and INR off-ramp corridors are illustrative UI stubs, not live SEP-24/SEP-31 anchors |
| `docs/` & `scripts/docs-check.cjs` | Technical specifications, VK commitment guide, threat/limitations documentation, and automated honesty checks | Testnet prototype documentation only; not legal or compliance advice |
