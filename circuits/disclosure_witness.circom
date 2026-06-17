pragma circom 2.2.0;

// Compute-only twin of disclosure.circom: same hashing, but the four public
// values are OUTPUTS instead of checked public inputs. Used only in dev to
// derive correct public signals (nullifierHash, commitment, discloseHash,
// auditorTag) for a given set of private inputs, so we can build a valid
// input.json for the real disclosure circuit without a separate Poseidon impl.

include "poseidon255.circom";

template DisclosureCompute() {
    signal input value;
    signal input label;
    signal input nullifier;
    signal input secret;
    signal input recipientId;
    signal input purpose;
    signal input viewingKey;

    signal output nullifierHash;
    signal output commitment;
    signal output discloseHash;
    signal output auditorTag;

    component nh = Poseidon255(1);
    nh.in[0] <== nullifier;
    nullifierHash <== nh.out;

    component pre = Poseidon255(2);
    pre.in[0] <== nullifier;
    pre.in[1] <== secret;

    component com = Poseidon255(3);
    com.in[0] <== value;
    com.in[1] <== label;
    com.in[2] <== pre.out;
    commitment <== com.out;

    component dh = Poseidon255(3);
    dh.in[0] <== recipientId;
    dh.in[1] <== purpose;
    dh.in[2] <== value;
    discloseHash <== dh.out;

    component at = Poseidon255(2);
    at.in[0] <== viewingKey;
    at.in[1] <== nh.out;
    auditorTag <== at.out;
}

component main = DisclosureCompute();
