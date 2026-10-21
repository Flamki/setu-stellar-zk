pragma circom 2.2.0;

// Auditor-side recomputation helper (witness-only; no proving, no secrets).
// The auditor knows the disclosed cleartext (recipientId, purpose, value), the
// viewingKey shared with them, and the receipt's public nullifierHash and
// receiptNonce. They recompute the two bound hashes and check them against the
// receipt's public signals [2],[3]. They never learn nullifier or secret.
// receiptNonce is public and travels with the receipt; it is the per-receipt
// nonce that separates repeated disclosures for the same withdrawal.

include "poseidon255.circom";

template AuditorRecompute() {
    signal input recipientId;
    signal input purpose;
    signal input value;
    signal input viewingKey;
    signal input nullifierHash;
    signal input receiptNonce;

    signal output discloseHash;
    signal output auditorTag;

    component dh = Poseidon255(3);
    dh.in[0] <== recipientId;
    dh.in[1] <== purpose;
    dh.in[2] <== value;
    discloseHash <== dh.out;

    component at = Poseidon255(3);
    at.in[0] <== viewingKey;
    at.in[1] <== nullifierHash;
    at.in[2] <== receiptNonce;
    auditorTag <== at.out;
}

component main = AuditorRecompute();
