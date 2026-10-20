# Public Signal & Proof Serialization Specification

**Status: prototype.** This document is the canonical byte-layout contract for
Groth16 `Proof` and `PublicSignals` as consumed on-chain by `libs/zk` and
produced by `cli/circom2soroban`. It matches the deserializer behavior in
`libs/zk/src/lib.rs`. Setu runs on Stellar **testnet**, is **not audited for
production**, and this encoding is **not** a general-purpose ZK interchange
format — it exists so withdrawal and disclosure proofs cannot be replayed via
non-canonical scalar encodings.

Related reading: [Privacy & Compliance Limitations](privacy-compliance-limitations.md).

---

## Curve and Field

| Parameter | Value |
| --- | --- |
| Curve | BLS12-381 |
| Scalar field | Fr (prime-order subgroup scalar field) |
| Scalar modulus `r` (big-endian bytes) | `73 ed a7 53 29 9d 7d 48 33 39 d8 08 09 a1 d8 05 53 bd a4 02 ff fe 5b fe ff ff ff ff 00 00 00 01` |
| Fr wire size | 32 bytes, **big-endian** integer in `[0, r)` |
| G1 wire size | `G1_SERIALIZED_SIZE` (96 bytes), uncompressed affine as used by Soroban BLS12-381 |
| G2 wire size | `G2_SERIALIZED_SIZE` (192 bytes), uncompressed affine as used by Soroban BLS12-381 |

Endianness for all length prefixes and Fr scalars is **big-endian**.
Little-endian Fr encodings are **malformed** for this protocol (they will either
fail length/canonical checks or decode to a different field element).

A scalar encoding is **canonical** iff its 32-byte big-endian integer is
**strictly less than** `r`. The modulus itself and every value `>= r` are
**non-canonical** and MUST be rejected with `NonCanonicalPublicSignal`.

---

## Proof byte layout

`Proof::{to_bytes, from_bytes}` use a fixed-length concatenation (no length
prefix):

```text
offset  size   field
------  -----  -----
0       96     A : G1Affine   (proof π_A)
96      192    B : G2Affine   (proof π_B)
288     96     C : G1Affine   (proof π_C)
------  -----
total   384
```

### Rejection cases (`Groth16Error::MalformedProof`)

| Case | Behavior |
| --- | --- |
| `bytes.len() != 384` | Reject (truncated or trailing garbage) |
| Length correct but callers pass non-curve bytes | Point construction may still fail later in verification; the serializer only enforces exact length |

There is **no** compressed-point variant and **no** version byte in this
prototype layout.

---

## PublicSignals byte layout

`PublicSignals::{to_bytes, from_bytes}` use a length-prefixed Fr vector:

```text
offset              size     field
------              -----    -----
0                   4        n : u32 big-endian (number of public signals)
4                   32       pub[0] : Fr big-endian, canonical
4 + 32              32       pub[1] : Fr big-endian, canonical
…                   …        …
4 + 32*(n-1)        32       pub[n-1]
------              -----
total               4 + 32*n
```

Exact-length rule: after reading `n`, the decoder REQUIRES

```text
bytes.len() == 4 + 32 * n
```

Any shorter **or longer** payload is `MalformedPublicSignals`. Overflow when
computing `32 * n` is also `MalformedPublicSignals`.

### Per-signal validation

For each 32-byte slice:

1. If the integer is `>= r` (including equality with `r`), reject with
   `NonCanonicalPublicSignal`.
2. Otherwise decode as `U256` big-endian → `Fr`.

This closes the audit finding where a prover could submit `x + k*r` so that
pairing math reduced to `x` while nullifier / set-membership compared raw bytes.

### Rejection summary

| Error | Trigger |
| --- | --- |
| `MalformedPublicSignals` | Fewer than 4 header bytes; truncated Fr lane; trailing bytes; length overflow |
| `NonCanonicalPublicSignal` | Any Fr lane with value `>= r` |

Empty vectors (`n = 0`, payload exactly 4 zero bytes for the length) are
syntactically valid at the serializer layer; circuit-specific verifiers still
require `pub_signals.len() + 1 == vk.ic.len()`.

---

## Circuit public-signal ordering

snarkjs / Circom emit public signals as **circuit outputs first**, then
**public inputs** in declaration order. On-chain `PublicSignals` MUST preserve
that order when serialized: `pub[0]` is the first snarkjs public value.

Machine-readable copies of both orders and their testnet values live in
`libs/zk/testdata/public_signals/{withdrawal,disclosure}.json`. The tests
`test_withdrawal_public_signals_from_fixture` and
`test_disclosure_public_signals_from_fixture` regenerate `PublicSignals` from
those fixtures and cross-check this document, the Circom sources, and the
README, so reordering a signal anywhere fails `cargo test -p zk`. The fixture
values are prototype data from the local/staging trusted setup; they pin
ordering, not ceremony quality.

### Withdrawal (`circuits/main.circom`)

```circom
component main {public [withdrawnValue, stateRoot, associationRoot]} = Withdraw(20, 2);
// plus signal output nullifierHash
```

Canonical order (`n = 4`):

| Index | Name | Role |
| --- | --- | --- |
| 0 | `nullifierHash` | Poseidon(nullifier); spent-marker / double-spend key |
| 1 | `withdrawnValue` | Amount withdrawn (range-checked in circuit) |
| 2 | `stateRoot` | Pool Merkle root |
| 3 | `associationRoot` | Association-set Merkle root |

Fresh testnet example (decimal Fr strings, same order as snarkjs `public.json`):

```json
[
  "33832171054643436472546998686772011210227251098487950275135154568712175384598",
  "1000000000",
  "30162851960749159054107963444341137279716337900493764726816877893946218126682",
  "30671046209969431012473152916297518771579159592633900587133061089753651787613"
]
```

Binary encoding of that vector starts with length prefix `00 00 00 04`, then
four 32-byte big-endian Fr encodings (leading zeros allowed; e.g.
`withdrawnValue = 1000000000` is
`00 … 00 3b 9a ca 00`).

### Disclosure (`circuits/disclosure.circom`)

```circom
component main {public [nullifierHash, commitment, discloseHash, auditorTag, receiptNonce]} = Disclosure();
```

Canonical order (v2; `n = 5`, all public inputs — no separate outputs):

| Index | Name | Role |
| --- | --- | --- |
| 0 | `nullifierHash` | Spent nullifier hash (must already be on-chain) |
| 1 | `commitment` | Deposit leaf opening under disclosure |
| 2 | `discloseHash` | Poseidon255(recipientId, purpose, value) |
| 3 | `auditorTag` | Poseidon255(viewingKey, nullifierHash, receiptNonce) |
| 4 | `receiptNonce` | Prover-chosen per-receipt nonce (public, not a secret) |

Fresh example (decimal Fr strings, same order as snarkjs `public.json`),
recomputed from the sample private inputs in `circuits/disclosure_private.json`
via the compute-only twin `circuits/disclosure_witness.circom`:

```json
[
  "50965195888132986870075188991270610090721666015373119467878104864524153103363",
  "13504266674507873070437848741371028790142490341642731696694125123435309057848",
  "34148362321686645833561023478297102534116878299856681891161633532616282480240",
  "48504087352648101872232104541849549896624195243212218591472176225635135984236",
  "31337"
]
```

Reordering these five values produces a different byte string and MUST fail
verification against the disclosure VK (or fail contract semantic checks even if
a forged layout somehow verified under a different key).

#### Receipt nonce and tag derivation (v2)

`receiptNonce` is the per-receipt domain separator that makes repeated
disclosures of the **same** withdrawal separable (issue #3):

- v1 tag: `auditorTag = Poseidon255(viewingKey, nullifierHash)` — deterministic
  per (key, withdrawal); every repeated disclosure produced an identical tag.
- v2 tag: `auditorTag = Poseidon255(viewingKey, nullifierHash, receiptNonce)` —
  the prover picks a fresh `receiptNonce` per receipt, so two receipts with the
  same `nullifierHash` and `viewingKey` carry different `auditorTag` values and
  cannot be matched by comparing tags alone.

The nonce is **public and not secret**: uniqueness is all it needs. It travels
with the receipt (like the disclosed cleartext) so the auditor can recompute
the tag with `circuits/auditor_recompute.circom`. The v1 and v2 tag values for
the same inputs never collide, because they are different Poseidon arities;
pinned examples live in `libs/zk/testdata/receipt_tags.json` and are checked by
`cargo test -p zk`.

Prototype limit: the circuit cannot force nonce freshness. A prover who reuses
a nonce reproduces the v1 linkability for that pair of receipts; freshness is a
prover-side discipline, not a proof guarantee.

#### Compatibility with v1 (pre-nonce) staging receipts

The earlier staging format had exactly four public signals
`[nullifierHash, commitment, discloseHash, auditorTag]` with the deterministic
v1 tag. Changing the circuit changed the verification key, so:

- v1 receipts verify **only** under the archived v1 key, and v2 receipts only
  under a v2 key. The Groth16 verifier requires
  `pub_signals.len() + 1 == vk.ic.len()`, so a wrong-length vector under a given
  key fails regardless of proof contents.
- `verify_disclosure` accepts a 4- or 5-signal vector as a cheap pre-filter and
  lets the installed key decide the format; historical v1 receipts are not
  silently accepted by a v2 key.
- Receipts produced by the pre-nonce testnet deployment
  (`docs/`, README "Fresh disclosure public signals" before this change) are
  v1 artifacts. Re-proving with the v2 circuit is required to use the nonce
  derivation; there is no in-place upgrade of an existing Groth16 proof.

---

## VerificationKey layout (reference)

Not required for the public-signal audit fix, but produced by the same
converter:

```text
alpha (G1) || beta (G2) || gamma (G2) || delta (G2) || ic_len (u32 BE) || ic[i] (G1)*
```

`ic.len()` MUST equal `nPublic + 1`.

---

## Conformance tests

The following tests in `libs/zk/src/test.rs` pin this specification:

| Test | Spec clause |
| --- | --- |
| `test_proof_serde` | Proof round-trip, 384-byte layout |
| `test_proof_serde_rejects_truncated_bytes` | MalformedProof on wrong length |
| `test_public_signals_serde` | Length prefix + Fr BE round-trip |
| `test_public_signals_reject_non_canonical_scalar` | Reject Fr encoding `== r` |
| `test_public_signals_reject_truncated_payload` | MalformedPublicSignals |
| `test_withdrawal_public_signal_order_encoding` | Withdrawal index order + `n=4` header |
| `test_disclosure_public_signal_order_encoding` | Disclosure index order + `n=4` header |
| `test_withdrawal_public_signals_from_fixture` | Fixture → signal regeneration, order cross-checks (withdrawal) |
| `test_disclosure_public_signals_from_fixture` | Fixture → signal regeneration, order cross-checks (disclosure) |
| `test_receipt_tag_nonce_separates_repeated_disclosures` | v2 `receiptNonce` separates repeated disclosures (issue #3) |
| `test_receipt_tag_viewing_key_and_nonce_both_change_tag` | `auditorTag` bound to viewing key and nonce |
| `test_receipt_tag_is_stable_for_fixed_nonce_and_key` | Pinned v2 tag for fixed (key, nonce) |
| `test_receipt_tag_v2_never_equals_v1_legacy_tag` | v1/v2 tag formats never collide |

Run:

```bash
cargo test -p zk
```

---

## Prototype limits (honest)

- Encoding is **uncompressed** BLS12-381 points only; no EIP-197 / compressed
  variants.
- Trusted setup is local/staging (see privacy limitations doc).
- Association-root zero-bypass exists in the withdrawal **circuit**; the
  contract mitigates it on-chain today.
- This document describes the Setu fork wire format; do not assume compatibility
  with BN254 snarkjs defaults or other Groth16 libraries without conversion.
