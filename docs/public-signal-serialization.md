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
component main {public [nullifierHash, commitment, discloseHash, auditorTag]} = Disclosure();
```

Canonical order (`n = 4`, all public inputs — no separate outputs):

| Index | Name | Role |
| --- | --- | --- |
| 0 | `nullifierHash` | Spent nullifier hash (must already be on-chain) |
| 1 | `commitment` | Deposit leaf opening under disclosure |
| 2 | `discloseHash` | Poseidon255(recipientId, purpose, value) |
| 3 | `auditorTag` | Poseidon255(viewingKey, nullifierHash) |

Fresh testnet example:

```json
[
  "33832171054643436472546998686772011210227251098487950275135154568712175384598",
  "792451850146572312119437015092516461585820411321474856545290648487812800938",
  "13601723215849916214344531109121559986847487952922840288412049033564696840611",
  "49445820628079692271178516198415444927595013704302380125940463497865958044604"
]
```

Reordering these four values produces a different byte string and MUST fail
verification against the disclosure VK (or fail contract semantic checks even if
a forged layout somehow verified under a different key).

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
