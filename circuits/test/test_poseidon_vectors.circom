pragma circom 2.2.0;

include "../poseidon255.circom";

// Exercise every arity supported by coinutils::poseidon_hash. Inputs are
// canonical BLS12-381 scalar field elements, not application byte strings.
template PoseidonVectors() {
    signal input inputs[3];
    signal output hash1;
    signal output hash2;
    signal output hash3;

    component h1 = Poseidon255(1);
    component h2 = Poseidon255(2);
    component h3 = Poseidon255(3);
    h1.in[0] <== inputs[0];
    for (var i = 0; i < 2; i++) {
        h2.in[i] <== inputs[i];
    }
    for (var i = 0; i < 3; i++) {
        h3.in[i] <== inputs[i];
    }
    hash1 <== h1.out;
    hash2 <== h2.out;
    hash3 <== h3.out;
}

component main = PoseidonVectors();
