# Circuits Test Suite

This directory contains compatiblity tests for Poseidon hash and Merkle-Tree implementations in Rust and Circom.

## Lean-IMT Compatibility Test

This directory contains a compatibility test that verifies the `merkleProof.circom` circuit works correctly with the Lean-IMT (Lean Incremental Merkle Tree) implementation from the Rust codebase.

### Overview

The Lean-IMT compatibility test ensures consistency between:
1. **Rust Implementation**: `lean-imt` crate with LeanIMT implementation
2. **Circom Implementation**: `merkleProof.circom` circuit

This is crucial for maintaining consistency between the Rust implementation (used in the main application) and the Circom implementation (used in zero-knowledge proofs).

### Test Files

#### 1. `test/test_merkleProof.circom`
The main test circuit that includes:
- `TestMerkleProof`: Tests merkle proof verification with configurable depth
- **Input signals**: `leaf`, `leafIndex`, `siblings`, `actualDepth`
- **Output signals**: `out` (computed merkle root)
- **Components**: 
  - `MerkleProof(maxDepth)` for merkle proof verification
  - Uses depth 2 for the main test component

#### 2. `test/lean-imt-test/`
Rust test implementation that:
- Uses the `lean-imt` crate with LeanIMT implementation
- Generates test data and merkle proofs
- Outputs a `circuit_input.json` file with test data

### Running the Lean-IMT Compatibility Test

#### Prerequisites
- Rust toolchain (cargo)
- circom compiler
- snarkjs

#### Step 1: Generate Test Data from Rust Implementation
```bash
cd circuits/test
cargo run --bin lean-imt-test --manifest-path lean-imt-test/Cargo.toml
```

This will generate a `circuit_input.json` file with test data including:
- `leaf`: The leaf value to prove inclusion of
- `leafIndex`: Index of the leaf in the Merkle tree
- `siblings`: Array of sibling values along the path to root
- `actualDepth`: Current tree depth

#### Step 2: Compile the Test Circuit
```bash
cd ../../
circom circuits/test/test_merkleProof.circom -l $CIRCOMLIB --wasm --prime bls12381 -o circuits/build/
```

#### Step 3: Generate Witness from Circom Circuit
```bash
cd circuits/build/test_merkleProof_js
node generate_witness.js test_merkleProof.wasm ../../test/circuit_input.json ../../test/test_merkleProof.wtns
```

#### Step 4: Extract Witness Outputs
```bash
cd ../../test
snarkjs wtns export json test_merkleProof.wtns test_merkleProof.wtns.json
```

#### Step 5: Verify Compatibility
Compare the computed root from the circuit with the expected root from the Rust implementation. The outputs should match, confirming that both implementations produce identical merkle proof verification results.

### Test Circuit Structure

The `test_merkleProof.circom` circuit:
- **Input signals**: `leaf`, `leafIndex`, `siblings`, `actualDepth`
- **Output signals**: `out` (computed merkle root)
- **Public signals**: All inputs and outputs are public for testing purposes
- **Merkle proof verification**: Uses the same hashing and verification logic as the Rust implementation

### Key Benefits

1. **Cross-Implementation Verification**: Ensures Rust and Circom implementations produce identical merkle proof verification results
2. **Cryptographic Consistency**: Validates that both implementations use the same hashing algorithms and tree construction
3. **Integration Testing**: Provides confidence that the two codebases can work together seamlessly
4. **Regression Prevention**: Catches any divergence between implementations during development

### Technical Details

- **Hash Function**: Both implementations use Poseidon255 for hashing
- **Tree Structure**: Both follow the LeanIMT design principles
- **Proof Format**: Both handle the same merkle proof structure and validation
- **Output Format**: Both produce field elements in the same representation

### Troubleshooting Lean-IMT Tests

#### Common Issues
1. **Compilation Errors**: Ensure circom and circomlib are properly installed
2. **Witness Generation Errors**: Check that the generated `circuit_input.json` matches the expected format
3. **Mismatched Outputs**: Verify that both implementations use the same hashing algorithms

#### Debugging
- Use the generated witness files to verify outputs manually
- Check that both implementations use the same field arithmetic
- Verify that the merkle proof structure is consistent between implementations

## Poseidon Compatibility Test

This directory also contains a manual compatibility test for the `poseidon255.circom` circuit, which compares hash outputs from the Rust `soroban-poseidon` crate with the Circom implementation. The automated regression suite is documented under [Poseidon Parity Vectors](#poseidon-parity-vectors).

### Overview

The Poseidon compatibility test ensures consistency between:
1. **Rust Implementation**: `soroban-poseidon` crate with BLS12-381 scalar field parameters
2. **Circom Implementation**: `poseidon255.circom` circuit

This is crucial for maintaining consistency between the Rust implementation (used in the main application) and the Circom implementation (used in zero-knowledge proofs).

### Test Files

#### 1. `test_poseidon.circom`
The main test circuit that includes:
- `TestPoseidon`: Tests both single and two-input hashing
- **Input signals**: `in1`, `in2`
- **Output signals**: `out1` (single input hash), `out2` (two input hash)
- **Components**:
  - `Poseidon255(1)` for single input hashing
  - `Poseidon255(2)` for two input hashing

#### 2. `poseidon-test/`
Rust test implementation that:
- Uses the `soroban-poseidon` crate with BLS12-381 scalar field parameters
- Takes JSON input with two values: `in1` and `in2`
- Computes the same hashes as the circuit

#### 3. `test_poseidon_input.json`
Test input data:
```json
{
    "in1": "51576823595707970152643159819788304363803754756066229172775779360774743019614",
    "in2": "51576823595707970152643159819788304363803754756066229172775779360774743019614"
}
```

### Running the Poseidon Compatibility Test

Run every command below from the repository root. The shell examples use Bash input redirection.

#### Prerequisites
- Rust toolchain (cargo)
- Node.js and npm
- circom compiler
- snarkjs 0.7.6 (`npm ci --ignore-scripts --prefix circuits/test`)

#### Step 1: Compute Hashes with the Rust Implementation
```bash
cargo run --locked --bin test_poseidon --manifest-path circuits/test/poseidon-test/Cargo.toml < circuits/test/test_poseidon_input.json
```

**Expected Output:**
```
23949694927245629364409973542171811196735943625447189117637854868491068555458
33646187916922823865935622258451714952164674255482660942215703235411158105736
```

#### Step 2: Generate Witness from Circom Circuit
```bash
mkdir -p circuits/build
circom circuits/test/test_poseidon.circom --prime bls12381 --r1cs --wasm --sym -o circuits/build
node circuits/build/test_poseidon_js/generate_witness.js circuits/build/test_poseidon_js/test_poseidon.wasm circuits/test/test_poseidon_input.json circuits/build/test_poseidon.wtns
```

#### Step 3: Check the Witness and Extract Outputs
```bash
node circuits/test/node_modules/snarkjs/build/cli.cjs wtns check circuits/build/test_poseidon.r1cs circuits/build/test_poseidon.wtns
node circuits/test/node_modules/snarkjs/build/cli.cjs wtns export json circuits/build/test_poseidon.wtns circuits/build/test_poseidon.wtns.json
node -e "const w = require('./circuits/build/test_poseidon.wtns.json'); console.log(w[1]); console.log(w[2]);"
```

#### Step 4: Verify Compatibility
The symbol file maps `main.out1` to witness index 1 and `main.out2` to index 2. Compare these printed values with the two Rust outputs:

| Hash Type | Rust Output | Circom Output | Status |
|-----------|-------------|---------------|---------|
| Single Input (`in1`) | `23949694927245629364409973542171811196735943625447189117637854868491068555458` | `23949694927245629364409973542171811196735943625447189117637854868491068555458` | ✅ **MATCH** |
| Two Inputs (`in1`, `in2`) | `33646187916922823865935622258451714952164674255482660942215703235411158105736` | `33646187916922823865935622258451714952164674255482660942215703235411158105736` | ✅ **MATCH** |

### Test Circuit Structure

The `test_poseidon.circom` circuit:
- **Input signals**: `in1`, `in2`
- **Output signals**: `out1`, `out2`
- **Public signals**: All inputs and outputs are public for testing purposes
- **Hash functions**: Uses the same Poseidon255 parameters as the Rust implementation

### Key Benefits

1. **Cross-Implementation Verification**: Ensures Rust and Circom implementations produce identical results
2. **Cryptographic Consistency**: Validates that both implementations use the same parameters and algorithms
3. **Integration Testing**: Provides confidence that the two codebases can work together seamlessly
4. **Regression Prevention**: Catches any divergence between implementations during development

### Technical Details

- **Field**: Both implementations use the BLS12-381 scalar field
- **Hash Function**: Poseidon255 with identical constants and parameters
- **Input Processing**: This example uses canonical decimal strings; it does not check arbitrary JSON-number conversions or invalid-input policies
- **Output Format**: Both produce field elements in the same representation

Prototype limit: this helper prints hashes for one manual sample and does not assert equality automatically. It covers one- and two-input hashing; the automated vector suite also covers three inputs and complete commitment derivations.

### Troubleshooting Poseidon Tests

#### Common Issues
1. **Compilation Errors**: Ensure circom and circomlib are properly installed
2. **Witness Generation Errors**: Check that input JSON matches the expected format
3. **Mismatched Outputs**: Verify that both implementations use the same Poseidon parameters

#### Debugging
- Use the generated witness files to verify outputs manually
- Check that both implementations use the same field arithmetic
- Verify Poseidon constants are identical between implementations

## Poseidon Parity Vectors

The checked-in fixture at `cli/coinutils/tests/fixtures/poseidon_vectors.json` contains 69 Circom-derived reference outputs. Rust tests call the production `coinutils::poseidon_hash` and `coinutils::generate_commitment` functions; expected outputs never come from Rust.

### Coverage and boundaries

- **Direct hashes:** 15 input triples, each hashed at arities 1, 2, and 3 (`t = arity + 1`), for 45 comparisons against `Poseidon255`.
- **Project derivations:** 12 fixed `(value, label, nullifier, secret)` tuples, yielding 12 commitments and 12 nullifier hashes from the existing `test_commitment.circom` / `CommitmentHasher` circuit. These exercise `Poseidon(value, label, Poseidon(nullifier, secret))` and `Poseidon(nullifier)`, including argument-order changes.
- **Inputs:** zero, one, distinct/reordered values, zero in each hash position, the BLS12-381 scalar modulus minus one, mixed zero/maximum values, values around 2^32, 2^64, and 2^128, and values with bit 254 set.
- All fixture numbers are decimal **strings**, preserving exact integers. Inputs are canonical field elements in `[0, r)`, where `r` is recorded in the fixture. The Rust fixture adapter rejects out-of-range values rather than relying on reduction.

The boundaries start at field elements. Production wrapper conversions and commitment composition are covered. CLI decimal parsing, scope/nonce byte preprocessing, Merkle-tree behavior, receipt-tag derivations, proof generation, and on-chain deposit/withdrawal execution are outside these tests.

### Run the Rust tests

No Circom or Node installation is needed to consume the checked-in fixture:

```sh
cargo test --locked -p coinutils --test poseidon_parity
```

A mismatch fails the test and reports the vector ID, derivation, and hash arity where applicable. Run `cargo test --locked -p coinutils` for the package suite.

### Regenerate or check the Circom reference

Use Circom **2.2.2**, Node.js **24**, and the locked snarkjs **0.7.6** dependency:

```sh
npm ci --ignore-scripts --prefix circuits/test
node scripts/generate-poseidon-vectors.cjs --check
node scripts/generate-poseidon-vectors.cjs
```

`--check` regenerates the reference and exits nonzero on any difference without rewriting the committed fixture. The command without `--check` replaces it; review the resulting diff before accepting changed outputs. Set `CIRCOM_BIN` when the compiler is not named `circom` on PATH.

The generator compiles the repository's actual circuits with `--prime bls12381 --O1 --r1cs --wasm --sym`, calculates witnesses, checks every witness against the R1CS constraints, and reads named outputs through the compiler's symbol files. It records source SHA-256 digests after normalizing CRLF to LF. No timestamps or machine-specific paths enter the fixture. Both Rust verification and fresh Circom regeneration run in `circuits-ci`.

To reproduce the eight deliberate mismatch checks, run `node scripts/check-poseidon-mismatches.cjs` with the same toolchain. It temporarily changes the fixture or Rust source, requires each mismatch to fail, restores the original bytes, and reruns the unmodified parity tests. Run it without concurrent edits to those files. CI runs this check after the package suite.

The older `test_poseidon.circom` and `poseidon-test` binary remain manual one-/two-input comparison tools; they are not used to generate these vectors.

Prototype limit: a finite set of matching vectors is regression evidence, not an exhaustive equivalence proof, a cryptographic audit, or a statement about trusted-setup quality. All inputs are public, synthetic examples.

See [the test results report](../../docs/poseidon-parity-test-report.md) for measured results, mismatch experiments, and remaining coverage limits.
