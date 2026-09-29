# Verification Key Commitments

The pool stores its Groth16 verification keys as opaque `Bytes`, so until now an
auditor had no cheap way to confirm **which** key a deployment actually verifies
with. This document describes the commitments the contract exposes and the
procedure for comparing them against the repo's circuit artifacts.

## What a commitment is (and is not)

A commitment is a SHA-256 fingerprint of the exact bytes held in storage:

```text
withdrawal key : sha256("setu:vk-commitment:v1"  || vk_bytes)
disclosure key : sha256("setu:dvk-commitment:v1" || dvk_bytes)
```

The domain tag is hashed in front of the key bytes. That means installing the
*same* key bytes in both slots produces two different commitments, so a
withdrawal-key commitment can never be mistaken for a disclosure-key one.

A commitment **is**:

- a stable identity for an installed key — recompute it locally and compare;
- cheap to publish, log and compare (32 bytes);
- collision-resistant: two different keys cannot share a commitment.

A commitment is **not**:

- a proof that the key is correct, or that it was derived from the circuit in
  this repository. It only tells you that the key you hold and the key the
  contract holds are the same bytes;
- a commitment to the trusted setup, the circuit's `.r1cs`, or the proving key.
  It says nothing about whether the setup ceremony was trustworthy (the current
  build uses a local/staging setup — see
  [Privacy & Compliance Limitations](privacy-compliance-limitations.md));
- proof that a deployment is using a *published* key at all. A malicious admin
  can install any key; the commitment just makes that visible and comparable.

## Reading the commitment from a deployment

```bash
stellar contract invoke --id <CONTRACT_ID> --source <IDENTITY> --network testnet \
  -- get_vk_commitment            # withdrawal key; None if unset

stellar contract invoke --id <CONTRACT_ID> --source <IDENTITY> --network testnet \
  -- get_disclosure_vk_commitment # disclosure key; None until set_disclosure_vk
```

Both return the raw 32 bytes. `get_disclosure_vk_commitment` returns `None`
until an admin calls `set_disclosure_vk`.

`set_disclosure_vk` also emits a `("dvk", "set")` event whose payload is the new
commitment, so key rotations are visible in the contract's event log without
re-reading storage.

## Recomputing it locally

The contract stores the bytes that `stellar-circom2soroban` produces from the
exported `vk.json`, so hash exactly those bytes — not the JSON text:

```bash
# 1. Export the verification key from the circuit artifact.
snarkjs zkey export verificationkey circuits/build/main_final.zkey main_vk.json
snarkjs zkey export verificationkey circuits/build_disc/disc_final.zkey disc_vk.json

# 2. Convert to the bytes the deploy script passes to the contract.
VK_HEX=$(cargo run -q --bin stellar-circom2soroban -- vk main_vk.json)

# 3. Hash domain || bytes and compare with get_vk_commitment.
node -e '
  const c = require("crypto");
  const hex = process.argv[2].replace(/^0x/, "").replace(/[^0-9a-fA-F]/g, "");
  const domain = Buffer.from("setu:vk-commitment:v1");           // "setu:dvk-commitment:v1" for the disclosure key
  console.log(c.createHash("sha256").update(Buffer.concat([domain, Buffer.from(hex, "hex")])).digest("hex"));
' "$VK_HEX"
```

The same hex-decoding step matters in any language: the commitment is over the
binary key, so hashing a JSON file or a hex *string* yields a different (and
useless) value.

The current build's own deploy helpers do this wiring:

- `scripts/build-circuits.sh` exports `main_verification_key.json` from the
  circuit setup;
- `scripts/live_testnet_e2e.ps1` converts it with `stellar-circom2soroban` and
  passes the hex as the constructor's `--vk_bytes`, and again for
  `set_disclosure_vk --dvk_bytes`.

## Tests

`contract/src/test.rs` covers the getter and event paths:

- the commitment equals an independently recomputed `sha256(domain || bytes)`;
- it is stable across reads and changes when a different key is installed;
- the disclosure commitment is `None` until `set_disclosure_vk`, then uses its
  own domain tag, so identical key bytes in both slots do not collide;
- `set_disclosure_vk` publishes exactly one event.

## Prototype limits

- The commitment does not bind a deployment to the circuits in this repo; it
  only makes the installed key comparable. Trust still comes from the deployer
  publishing the same commitment out of band.
- The trusted setup is local/staging-only, so a matching commitment does **not**
  imply a secure setup.
- Nothing here changes the deposit or withdrawal paths; the pool's behaviour is
  unchanged apart from one extra event on disclosure-key installation.
