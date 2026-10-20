pragma circom 2.2.0;

// Setu selective-disclosure receipt circuit.
// Lives inside the fork so it reuses Poseidon255 (BLS12-381) + circom2soroban,
// and is byte-compatible with the pool's on-chain hashing.
//
// NOT in upstream soroban-privacy-pools. Their ASP proves "my deposit is in a
// clean set." This proves the complementary auditor-facing statement:
//   "The on-chain withdrawal with this nullifierHash spends this exact deposit
//    leaf, whose committed value is disclosed alongside prover-asserted
//    recipient/purpose context."
// Per-transaction reveal, no pool-wide unlink, secret never exposed.
//
// Commitment scheme matched to circuits/commitment.circom:
//   nullifierHash = Poseidon255(nullifier)
//   precommitment = Poseidon255(nullifier, secret)
//   commitment    = Poseidon255(value, label, precommitment)
//
// PRIVACY NOTE: this receipt deliberately links nullifierHash <-> commitment,
// which the pool otherwise keeps unlinkable. That link is revealed ONLY to the
// verifier of this proof (the auditor). The receipt must be transmitted
// privately; auditorTag lets an auditor recognize receipts intended for the
// viewing key they hold. It is not an authenticated auditor identity.
//
// RECEIPT NONCE (v2): auditorTag = Poseidon255(viewingKey, nullifierHash,
// receiptNonce). The prover picks a fresh receiptNonce per receipt, so two
// receipts for the SAME withdrawal (same nullifierHash, same viewingKey) get
// different tags and cannot be matched by comparing auditorTag alone. The
// nonce is PUBLIC: it is not secret, it only needs to be unique per receipt;
// it travels with the receipt so the auditor can recompute the tag. v1 tags
// were Poseidon255(viewingKey, nullifierHash) — deterministic per
// (key, withdrawal) and therefore linkable across repeated disclosures. v1
// receipts (4 public signals) are a separate circuit version: they verify only
// under the archived v1 key, not under this circuit's key. Prototype limit:
// the circuit cannot force nonce freshness — a prover who reuses a nonce
// reproduces the old linkability. See docs/public-signal-serialization.md.

include "poseidon255.circom";

template Disclosure() {
    // ---- PUBLIC ----
    signal input nullifierHash;   // also the on-chain spent marker
    signal input commitment;      // on-chain leaf; auditor checks get_commitments()
    signal input discloseHash;    // = Poseidon255(recipientId, purpose, value)
    signal input auditorTag;      // = Poseidon255(viewingKey, nullifierHash, receiptNonce)
    signal input receiptNonce;    // prover-chosen, unique per receipt (not a secret)

    // ---- PRIVATE ----
    signal input value;
    signal input label;
    signal input nullifier;
    signal input secret;
    signal input recipientId;     // prover-asserted beneficiary/context hash
    signal input purpose;         // prover-asserted purpose code
    signal input viewingKey;      // shared with the chosen auditor

    // 1. Tie the receipt to a real, settled withdrawal.
    component nh = Poseidon255(1);
    nh.in[0] <== nullifier;
    nullifierHash === nh.out;

    // 2. Reconstruct the commitment -> proves knowledge of this coin opening.
    component pre = Poseidon255(2);
    pre.in[0] <== nullifier;
    pre.in[1] <== secret;

    component com = Poseidon255(3);
    com.in[0] <== value;
    com.in[1] <== label;
    com.in[2] <== pre.out;
    commitment === com.out;

    // 3. Bind disclosed fields. The amount disclosed IS the committed value.
    //    recipientId/purpose are hashed context, but v1 does not prove they
    //    were committed at deposit time.
    component dh = Poseidon255(3);
    dh.in[0] <== recipientId;
    dh.in[1] <== purpose;
    dh.in[2] <== value;
    discloseHash === dh.out;

    // 4. Bind the receipt to a viewing key AND to this specific disclosure
    //    event. receiptNonce is the per-receipt domain separator: the auditor
    //    who knows viewingKey and is given receiptNonce (public, travels with
    //    the receipt) can recompute and confirm the tag, while two receipts
    //    for the same withdrawal with different nonces produce different tags.
    component at = Poseidon255(3);
    at.in[0] <== viewingKey;
    at.in[1] <== nullifierHash;
    at.in[2] <== receiptNonce;
    auditorTag === at.out;
}

component main {public [nullifierHash, commitment, discloseHash, auditorTag, receiptNonce]} = Disclosure();
