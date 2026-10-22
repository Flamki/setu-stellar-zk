use coinutils::{generate_commitment, poseidon_hash};
use num_bigint::BigUint;
use serde::Deserialize;
use soroban_sdk::{crypto::bls12_381::Fr as BlsScalar, BytesN, Env};

const MODULUS: &str =
    "52435875175126190479447740508185965837690552500527637822603658699938581184513";

#[derive(Deserialize)]
struct Vectors {
    schema_version: u32,
    field: String,
    modulus: String,
    hashes: Vec<HashVector>,
    commitments: Vec<CommitmentVector>,
}

#[derive(Deserialize)]
struct HashVector {
    id: String,
    inputs: [String; 3],
    outputs: [String; 3],
}

#[derive(Deserialize)]
struct CommitmentVector {
    id: String,
    value: String,
    label: String,
    nullifier: String,
    secret: String,
    commitment: String,
    nullifier_hash: String,
}

fn vectors() -> Vectors {
    let vectors: Vectors =
        serde_json::from_str(include_str!("fixtures/poseidon_vectors.json")).unwrap();
    assert_eq!(vectors.schema_version, 1);
    assert_eq!(vectors.field, "BLS12-381 scalar");
    assert_eq!(vectors.modulus, MODULUS);
    assert!(!vectors.hashes.is_empty());
    assert!(!vectors.commitments.is_empty());
    vectors
}

// Fixture adapter only: preserve the exact canonical field integer. The test
// boundary excludes the CLI's decimal parser and scope/nonce byte preprocessing.
fn scalar(env: &Env, decimal: &str) -> BlsScalar {
    let value = BigUint::parse_bytes(decimal.as_bytes(), 10).unwrap();
    assert_eq!(value.to_str_radix(10), decimal, "non-canonical decimal");
    assert!(value < BigUint::parse_bytes(MODULUS.as_bytes(), 10).unwrap());
    let bytes = value.to_bytes_be();
    let mut padded = [0u8; 32];
    padded[32 - bytes.len()..].copy_from_slice(&bytes);
    BlsScalar::from_bytes(BytesN::from_array(env, &padded))
}

fn decimal(value: &BlsScalar) -> String {
    BigUint::from_bytes_be(&value.to_bytes().to_array()).to_str_radix(10)
}

#[test]
fn poseidon_matches_circom_vectors() {
    let env = Env::default();
    env.cost_estimate().budget().reset_unlimited();
    for vector in vectors().hashes {
        let inputs = vector.inputs.map(|input| scalar(&env, &input));
        for arity in 1..=3 {
            let actual = poseidon_hash(&env, &inputs[..arity]);
            assert_eq!(
                decimal(&actual),
                vector.outputs[arity - 1],
                "Circom/Rust Poseidon mismatch: case={}, arity={arity}",
                vector.id
            );
        }
    }
}

#[test]
fn commitments_and_nullifiers_match_circom_vectors() {
    let env = Env::default();
    env.cost_estimate().budget().reset_unlimited();
    for vector in vectors().commitments {
        let value = scalar(&env, &vector.value);
        let label = scalar(&env, &vector.label);
        let nullifier = scalar(&env, &vector.nullifier);
        let secret = scalar(&env, &vector.secret);
        let commitment = generate_commitment(&env, value, label, nullifier.clone(), secret);
        assert_eq!(
            decimal(&commitment),
            vector.commitment,
            "Circom/Rust commitment mismatch: case={}",
            vector.id
        );
        assert_eq!(
            decimal(&poseidon_hash(&env, &[nullifier])),
            vector.nullifier_hash,
            "Circom/Rust nullifier hash mismatch: case={}",
            vector.id
        );
    }
}
