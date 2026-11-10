## Summary

<!-- Briefly describe what this PR changes and why. -->

Closes #<!-- issue number -->

## Affected Area(s)

- [ ] `contract/` — Soroban privacy pool or selective-disclosure verifier
- [ ] `circuits/` — Circom circuits (`main.circom`, `disclosure.circom`, `auditor_recompute.circom`)
- [ ] `libs/zk/` or `libs/lean-imt/` — Groth16 verifier, wire serialization, or Merkle tree
- [ ] `cli/` — `circom2soroban` or `coinutils`
- [ ] `site/` — Static remittance & auditor workspace UI / smoke checks
- [ ] `docs/` or `scripts/` — Technical documentation, threat model, or CI checks

## Verification & Evidence Checklist

<!-- Paste command output or check the boxes for every check you ran locally. -->

- [ ] `cargo test --workspace` passes (required for any `contract/`, `libs/`, or `cli/` changes)
- [ ] `node scripts/docs-check.cjs` passes (required for any `README.md`, `docs/`, or `site/index.html` changes)
- [ ] `npm --prefix site run smoke` passes (required for any `site/` changes)

### Test Output / Proof of Verification

```text
Paste relevant test or smoke-check output here.
```

## Prototype Honesty & Privacy Impact

- [ ] **No overclaiming:** Comments, UI copy, and docs remain honest about Setu's testnet prototype limits (single-party trusted setup, mock USDC/INR anchor stubs, prover-asserted `recipientId`/`purpose` in v1/v2, and no legal/KYC compliance attestation).
- [ ] **Wire & signal compatibility:** If public signals or VK formats changed, `docs/public-signal-serialization.md` and `libs/zk/testdata/` fixtures were updated in lockstep.
- [ ] **Leakage review:** No private witness fields (`secret`, `nullifier`, `viewingKey`, Merkle path siblings) are logged, emitted in contract events, or copied to the clipboard.
