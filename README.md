# Setu

Private remittances on Stellar with compliance-oriented selective disclosure.

Setu is a hackathon build for the Stellar Real-World ZK track. It turns the
`soroban-privacy-pools` prototype into a remittance-focused flow:

1. Deposit a fixed-value note into a Soroban privacy pool.
2. Withdraw with a Groth16 proof of Merkle membership, nullifier correctness,
   and association-set membership.
3. Generate a selective-disclosure receipt proving that a spent nullifier links
   to a real deposited commitment and that the disclosed amount equals the
   committed value.
4. Let an auditor verify the receipt without learning the note secret.

The product story is cross-border stablecoin remittance: payment privacy for
families and small businesses, with an explicit compliance edge instead of a
black-box mixer.

## Live Testnet Deployment

- Contract: `CDXLQFYQJVDXBZDI5QVYRAM5TGPMZQWS424FCQWYVNGSKSSHPU6XXAXT`
- Lab link:
  https://lab.stellar.org/r/testnet/contract/CDXLQFYQJVDXBZDI5QVYRAM5TGPMZQWS424FCQWYVNGSKSSHPU6XXAXT
- WASM upload tx:
  https://stellar.expert/explorer/testnet/tx/c9c8ed3e1f6b988b76b29526967195aec4b135dc4bc29ad6fabde0073f701b51
- Contract deploy tx:
  https://stellar.expert/explorer/testnet/tx/3b9a86af7273750b54324c96fb3129a7677f59b7eb288b3c5de6dfe60abb019a
- Fresh deposit tx:
  https://stellar.expert/explorer/testnet/tx/adf901702fea3fcd09e0b8a94d19388d23d7579d05cbe8f7210b30489f9eb458
- Fresh association-root tx:
  https://stellar.expert/explorer/testnet/tx/709cf0d7530259e33b0af6ba0aa46d811947b1aa71a33f66042e275dfaafa938
- Fresh withdrawal tx:
  https://stellar.expert/explorer/testnet/tx/6d19b21f5f96ac4237ce85882c81c9d991cd88cfb4859fd59c359b28463cfb9a
- Disclosure VK install tx:
  https://stellar.expert/explorer/testnet/tx/d35c95b0c65ddaedee2b85cd52a5d248571abb36d56c42c3599235ef8d64c6e0

Fresh withdrawal public signals:

```json
[
  "33832171054643436472546998686772011210227251098487950275135154568712175384598",
  "1000000000",
  "30162851960749159054107963444341137279716337900493764726816877893946218126682",
  "30671046209969431012473152916297518771579159592633900587133061089753651787613"
]
```

Fresh disclosure public signals:

```json
[
  "33832171054643436472546998686772011210227251098487950275135154568712175384598",
  "792451850146572312119437015092516461585820411321474856545290648487812800938",
  "13601723215849916214344531109121559986847487952922840288412049033564696840611",
  "49445820628079692271178516198415444927595013704302380125940463497865958044604"
]
```

On the fresh patched deployment:

- `snarkjs groth16 verify` for the disclosure proof returned `OK!`.
- `verify_disclosure(validProof, validPublicSignals)` returned `true`.
- `verify_disclosure(validProof, tamperedPublicSignals)` returned `false`.
- The withdrawal nullifier is stored on-chain:
  `4acc5489ab80200caae1eca0b44dd2335e92931ff51e358e4fc7381378367816`.

## What Is New In Setu

This repository is built on top of
[`ymcrcat/soroban-privacy-pools`](https://github.com/ymcrcat/soroban-privacy-pools)
(MIT). The fork already had the base privacy pool, BLS12-381 Groth16 verifier,
Poseidon parity, LeanIMT tree, nullifiers, association-set proof path, and
Circom-to-Soroban artifact conversion.

Setu adds:

- `circuits/disclosure.circom`: selective-disclosure receipt circuit.
- `circuits/disclosure_witness.circom`: helper circuit for deriving receipt
  public signals without a separate Poseidon implementation.
- `circuits/auditor_recompute.circom`: auditor-side recomputation helper.
- `contract/src/disclosure.rs`: `set_disclosure_vk`, `has_disclosure_vk`, and
  `verify_disclosure`.
- Safe proof/public-signal parsing in `libs/zk/src/lib.rs`, including
  canonical BLS12-381 scalar validation.
- Regression tests for malformed input and non-canonical public signals.
- `scripts/live_testnet_e2e.ps1`: reproducible patched testnet flow.

## ZK Statements

### Withdrawal Circuit

`circuits/main.circom` proves:

- The prover knows a deposited note opening.
- The note commitment is included in the pool Merkle tree.
- The nullifier hash is correctly derived.
- The withdrawn value is consistent with the fixed-denomination note.
- The note label is included in the current association set root.

The contract additionally checks:

- The association root in the proof matches the admin-set root.
- The Merkle root in the proof matches the current pool root.
- The nullifier has not already been used.
- The Groth16 proof verifies under the stored withdrawal VK.

### Disclosure Circuit

`circuits/disclosure.circom` uses public signals:

```text
[nullifierHash, commitment, discloseHash, auditorTag]
```

and private witness:

```text
value, label, nullifier, secret, recipientId, purpose, viewingKey
```

It proves:

- `nullifierHash = Poseidon255(nullifier)`.
- `commitment = Poseidon255(value, label, Poseidon255(nullifier, secret))`.
- `discloseHash = Poseidon255(recipientId, purpose, value)`.
- `auditorTag = Poseidon255(viewingKey, nullifierHash)`.

`verify_disclosure` additionally checks:

- `nullifierHash` is already spent in this pool.
- `commitment` is a real deposited leaf in this pool.
- The receipt proof verifies under the disclosure VK.

Important v1 limit: `value` is cryptographically tied to the deposited
commitment. `recipientId` and `purpose` are prover-asserted context hashed into
the receipt; they are not deposit-time committed facts. A production-grade
version should bind those fields into the deposit commitment or verify an
off-ramp signature over them.

## Security Fix From Audit

The audit found a serious cross-layer issue: public signals were parsed into
field elements while set-membership checks used raw bytes. A proof could be
replayed with a non-canonical encoding such as `n + r`, where the pairing
reduces mod `r` but the nullifier set sees a different byte string.

Setu fixes this by:

- Making `Proof::from_bytes` and `PublicSignals::from_bytes` return `Result`.
- Rejecting malformed/truncated proof and public-signal byte arrays.
- Rejecting any BLS12-381 scalar public signal greater than or equal to the
  scalar modulus.
- Returning normal proof-failure responses instead of trapping.

Regression tests cover:

- Truncated proof bytes.
- Truncated public-signal payloads.
- Non-canonical public signals.
- Disclosure verifier malformed inputs.
- Withdraw rejecting non-canonical public signals without storing a nullifier.

## Run Locally

Prerequisites:

- Rust stable.
- Stellar CLI 26.x.
- Node.js.
- `circom` 2.2.x.
- `snarkjs` 0.7.x.
- `circomlib`.

Install JS dependencies from the parent workspace if needed:

```powershell
cd C:\Users\bbook\Desktop\zk
npm install
```

From this repository:

```powershell
cd C:\Users\bbook\Desktop\zk\fork
cargo test
cargo build --target wasm32v1-none --release -p privacy-pools
```

Verify the regenerated disclosure proof:

```powershell
node C:\Users\bbook\Desktop\zk\node_modules\snarkjs\build\cli.cjs groth16 verify `
  circuits\build_disc\disc_vk.json `
  circuits\build_disc\public.json `
  circuits\build_disc\proof.json
```

Run a fresh patched testnet flow:

```powershell
.\scripts\live_testnet_e2e.ps1
```

The script deploys if no `-ContractId` is supplied. To reuse the current live
contract:

```powershell
.\scripts\live_testnet_e2e.ps1 `
  -ContractId CDXLQFYQJVDXBZDI5QVYRAM5TGPMZQWS424FCQWYVNGSKSSHPU6XXAXT
```

## Project Structure

```text
contract/                  Soroban pool + disclosure verifier
libs/zk/                   BLS12-381 Groth16 verifier and serializers
libs/lean-imt/             Lean incremental Merkle tree
cli/circom2soroban/        snarkjs artifact conversion
cli/coinutils/             note generation and witness input generation
circuits/main.circom       withdrawal proof circuit
circuits/disclosure.circom selective-disclosure receipt circuit
scripts/live_testnet_e2e.ps1
```

## What Is Mocked

- Fiat on-ramp and INR off-ramp are product-story stubs, not live anchor
  integrations.
- Testnet native asset is used as the demo token.
- The trusted setup is local/dev-only and not production-secure.
- There is no relayer, so gas metadata privacy is future work.

## Future Work

- Bind recipient and purpose into the deposit commitment.
- Add authenticated auditor registry or verifier-key commitments.
- Use per-receipt nonce/key derivation for auditor tags.
- Add relayers for withdrawal gas privacy.
- Support multiple denominations or variable amounts.
- Replace local trusted setup with a real ceremony.
- Integrate real Stellar anchors for SEP-24/SEP-31 corridor demos.

## License And Attribution

Base privacy-pool implementation: `ymcrcat/soroban-privacy-pools`, MIT.

Setu additions are hackathon prototype code. This is not audited production
software and is not legal, compliance, or financial advice.
