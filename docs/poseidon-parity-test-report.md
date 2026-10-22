# Poseidon parity test design and results — issue #28

Date: 2026-10-22 UTC.

## Results

All **69 Circom/Rust output comparisons** passed: 45 direct Poseidon hashes, 12 complete commitments, and 12 nullifier hashes. The full `coinutils` package passed **16 tests**. All **8 deliberate mismatches** were detected, and independently regenerated reference vectors matched the saved fixture.

## Coverage and input boundaries

| Level | Circom reference | Rust consumer | Comparisons |
| --- | --- | --- | ---: |
| Direct Poseidon, arities 1/2/3 | `test_poseidon_vectors.circom` using the repository's `Poseidon255` | Production `coinutils::poseidon_hash` | 45 |
| Complete nested commitment | `test_commitment.circom` / `CommitmentHasher` | Production `coinutils::generate_commitment` | 12 |
| Nullifier hash | `CommitmentHasher` output | Production `coinutils::poseidon_hash` with one nullifier | 12 |

The commitment formula is `Poseidon(value, label, Poseidon(nullifier, secret))`; the nullifier hash is `Poseidon(nullifier)`.

The corpus contains 15 primitive input triples and 12 note tuples. It covers zero, one, distinct/reordered inputs, zero in each primitive input position, values around `2^32`, `2^64`, and `2^128`, bit 254, `r - 1`, values near `r - 1`, and mixed zero/maximum inputs. It also includes the documented disclosure sample `(100, 42, 12345, 67890)`.

The input boundary is **canonical BLS12-381 scalar field elements**, where:

```text
r = 52435875175126190479447740508185965837690552500527637822603658699938581184513
0 <= input < r
```

Fixture numbers are decimal strings. A test-only BigUint adapter checks the range and constructs exact scalar bytes. Tests exercise the production Rust hash wrapper and commitment composition, including argument ordering. They do not pass through CLI decimal parsing or scope/nonce byte preprocessing; the commitment circuit receives an already-derived `label`.

## Reproduce from a checkout of this change

### Rust tests

Install Rust 1.90.0 and the native linker required by your Rust target. Run these commands from the repository root:

```sh
cargo test --locked -p coinutils --test poseidon_parity
cargo test --locked -p coinutils
```

The first command runs two tests containing the 69 parity comparisons. The second runs all 16 package tests: 11 library unit tests, three existing integration tests, and the two parity tests. These commands consume the saved fixture and do not require Node.js or Circom.

### Regenerate and verify the Circom reference

Install Node.js 24 and [Circom 2.2.2](https://github.com/iden3/circom/releases/tag/v2.2.2). Ensure `circom` is on PATH, or set `CIRCOM_BIN` to the compiler executable. snarkjs 0.7.6 is installed from the repository's npm lockfile:

```sh
npm ci --ignore-scripts --prefix circuits/test
node scripts/generate-poseidon-vectors.cjs --check
```

Expected result:

```text
Checked 45 Poseidon outputs against Circom constraints.
PASS: regenerated vectors match the checked-in fixture.
```

The generator compiles the repository's circuits with `--prime bls12381 --O1 --r1cs --wasm --sym`. It calculates all **27 witnesses**, checks each against the compiled R1CS constraints, and reads named outputs through the compiler's symbol files. A witness assigns values to circuit signals; checking it confirms those values satisfy the constraints. Expected outputs are generated independently of Rust.

To intentionally update the reference after changing circuits, run:

```sh
node scripts/generate-poseidon-vectors.cjs
```

Review the fixture diff before accepting changed expected outputs. The fixture records field parameters, tool versions, compiler flags, and SHA-256 digests of circuit sources with LF-normalized line endings. Its generation uses a fixed corpus and contains no timestamps or machine-specific paths. The fixture's SHA-256 with LF line endings is:

```text
997d05191027e82fcda73890af82b20bed2529f398cb62096bef9100a40ad2af
```

### Verify that mismatches fail

With Rust, Node.js, Circom, and the npm dependencies above available, run:

```sh
node scripts/check-poseidon-mismatches.cjs
```

This command checks the unmodified parity tests first, then introduces the eight changes below one at a time. Each case must fail with the expected exit code and diagnostic. The script restores the original file bytes after each case and reruns the unmodified parity tests at the end. It temporarily edits the fixture and two Rust source files, so run it without other processes editing those files concurrently.

| Deliberate mismatch | Cases | Required failure |
| --- | ---: | --- |
| Increment the zero vector's expected hash at each arity | 3 | Cargo exit 101; `case=zero` and the affected arity |
| Increment an expected commitment or nullifier hash | 2 | Cargo exit 101; the affected derivation and `case=zero` |
| Reverse input iteration in the Rust Poseidon wrapper | 1 | Cargo exit 101; `case=distinct, arity=2` |
| Swap value/label in the Rust commitment calculation | 1 | Cargo exit 101; commitment `case=distinct` |
| Corrupt a saved expected hash and run regeneration with `--check` | 1 | Generator exit 1; reference fixture difference |

Expected final result:

```text
PASS: all 8 mismatches detected; original files restored; unmodified parity tests pass.
```

### Manual compatibility example

The [Poseidon Compatibility Test instructions](../circuits/test/README.md#poseidon-compatibility-test) retain the original single-/two-input manual workflow with the current sample and verified expected outputs. Both outputs matched, and the generated witness satisfied its constraints.

## Verification summary

| Command or check | Observed result |
| --- | --- |
| `cargo test --locked -p coinutils --test poseidon_parity` | 2 passed; 69 comparisons matched |
| `cargo test --locked -p coinutils` | 16 passed; 0 failed |
| `node scripts/generate-poseidon-vectors.cjs --check` | Reference matched; 27 witnesses satisfied the constraints |
| Independent reference regeneration | Identical fixture SHA-256 |
| `node scripts/check-poseidon-mismatches.cjs` | All 8 mismatches detected; restored tests passed |
| Manual compatibility example | Both Rust/Circom outputs matched; witness constraints passed |

The [`poseidon-parity` CI job](../.github/workflows/circuits-ci.yml) installs the specified tools and runs reference regeneration, the package suite, and mismatch detection. These instructions and scripts can also be run outside GitHub Actions. Hosted CI results are reported on the pull request.

## Limits

These finite vectors provide regression evidence for the stated boundaries. They are not exhaustive equivalence proofs or a cryptographic audit. They do not verify invalid/out-of-field input policy, CLI preprocessing, Merkle trees, receipt-tag derivations, Groth16 proof generation, contract execution, or ceremony quality. All fixture inputs are public synthetic test data.
