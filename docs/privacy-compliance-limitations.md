# Privacy & Compliance Limitations

**Status: prototype.** Setu is an active-development prototype that runs against
the Stellar **testnet**. It is **not audited production software**, it is **not a
regulated money transmitter or remittance provider**, and it is **not legal,
compliance, or financial advice**. Read this document before relying on any Setu
claim in the product story, README, or landing page.

This document is the counterpart to the product story in the README. The story
is about what Setu is *building toward*; this page is about what the code
actually proves today, and what it does **not** prove.

---

## What the Proof Proves

Setu has two proofs, verified by the Soroban contract:

### 1. Withdrawal proof (`circuits/main.circom`, verified in `withdraw`)

A valid withdrawal proof, combined with the contract's own checks, proves that:

- The prover knows the opening of a deposited note: `label`, `value`,
  `nullifier`, and `secret` (`circuits/commitment.circom`).
- The note commitment is a leaf in the pool's Merkle tree (`stateRoot`
  matches the commitment's computed Merkle path).
- `nullifierHash` is correctly derived from the note's `nullifier`
  (`nullifierHash = Poseidon(nullifier)`).
- The withdrawn value does not exceed the committed value (range-checked).
- The note's label` is a member of the current association set
  (`associationRoot`).

The contract additionally enforces, independent of the proof:

- The `associationRoot` in the proof equals the admin-set root.
- The Merkle `stateRoot` in the proof equals the contract's current pool root.
- The `nullifierHash` has not been used before (double-spend prevention).
- The Groth16 proof verifies under the stored withdrawal verification key.
- The contract transfers exactly the fixed denomination (`FIXED_AMOUNT`).

### 2. Selective-disclosure receipt (`circuits/disclosure.circom`, verified in `verify_disclosure`)

A valid receipt proof (public signals
`[nullifierHash, commitment, discloseHash, auditorTag]`) proves that:

- `nullifierHash = Poseidon255(nullifier)` for a private `nullifier` the
  prover knows.
- `commitment = Poseidon255(value, label, Poseidon255(nullifier, secret))`
  — i.e. the prover knows the full opening of the deposited note, including
  `value`.
- `discloseHash = Poseidon255(recipientId, purpose, value)` — the disclosed
  amount, recipient tag, and purpose are the exact values hashed into the
  receipt.
- `auditorTag = Poseidon255(viewingKey, nullifierHash)` — the receipt is bound
  to a viewing key the prover knows.

`verify_disclosure` additionally checks, against the contract's own state:

- `nullifierHash` is an **already-spent** withdrawal in ***this*** pool.
- `commitment` is a **real deposited leaf** in **this** pool.
- The receipt proof verifies under the disclosure verification key.

So a receipt proves: *"the on-chain withdrawal identified by this spent
nullifier spent this exact deposit leaf, whose committed value is the amount
disclosed in this receipt."*

---

## Withdrawal Event

After a successful proof verification and transfer, the contract emits a
withdrawal event with safe public metadata:

- `nullifierHash`: the spent-nullifier hash. This is already revealed by the
  withdrawal proof's public signals, so the event adds no new linkability.
- `amount`: the fixed withdrawal denomination (`FIXED_AMOUNT`), already public
  in the transfer.

This event is emitted ** only ** after the proof verifies, the nullifier is
unused, and the transfer succeeds. A duplicate/nullifier failure, failed proof,
or failed transfer does **not** emit the success event.

Privacy tradeoff: indexing `nullifierHash` makes it easier to track that a
specific withdrawal request settled. It does not reveal the note's `nullifier`,
`secret`, or which deposit leaf was spent; that link remains available only
through the selective-disclosure receipt.

---

## What the Proof Does Not Prove

The following are **not** established by any Setu proof. This is the part that
matters most for compliance claims.

- **Recipient and purpose are not deposit-time facts.** In v1, `recipientId`
  and `purpose` are *prover-asserted context* hashed into the receipt at
  withdrawal time. The circuit does not prove they were committed when the
  deposit was made. A production version must bind these fields into the
  deposit commitment or verify an off-ramp signature over them.
- **The auditor tag is not an authenticated identity.** `auditorTag` lets a
  holder of the matching `viewingKey` recognize a receipt intended for them.
  It does not prove the auditor is a licensed or registered entity, and it is
  not a verifiable credential.
- `**Nothing links the receipt to a real-world person.**` No proof connects the
  deposit or withdrawal to a KYC'd identity, a bank account, or a regulated
  off-ramp. The proofs are purely cryptographic statements about on-chain
  state.
- **The receipt does not prove the off-chain payout happened.** A valid
  receipt proves the disclosed amount equals the committed value of a real
  deposit; it says nothing about whether fiat was actually delivered to the
  recipient.
- **Privacy is limited, not absolute.** The disclosure receipt deliberately
  links `nullifierHash <-> commitment`, a link the pool otherwise keeps
  unlinkable. `verify_disclosure` on-chain publishes that link to **everyone**
  who reads the ledger — it is not a private channel. For a single regulator,
  prefer off-chain verification. The withdrawal proof itself does not reveal
  which deposit was spent, but it does not hide the withdrawing address, the
  amount, or the fact that a withdrawal occurred.
- **No gas/network-metadata privacy.** There is no relayer, so the sender's
  network identity is visible in transaction metadata. This is explicitly
  future work.
- **The circuit has a zero-root association bypass.** In
  `circuits/main.circom`, the association-membership constraint is skipped when
  `associationRoot == 0` (a backward-compatibility path). The contract blocks
  this on-chain by refusing withdrawals until a non-zero association root is
  set, so live withdrawals always carry a real root — but the circuit alone
  would accept the bypass, and the pattern should be removed before any
  production deployment.

---

## Relayer Flow Design (Draft)

**Status: design only. No relayer is implemented in this repository, and no
implementation claim is made until it is tested on testnet.** The section
below documents the options considered for Soroban, the decisions taken for
follow-up work, and the privacy gains and limits that must be honestly
represented once code exists.

### Problem

Today the withdrawing account signs and submits the transaction that calls
the pool's `withdraw` entrypoint. The ZK proof hides the note opening and the
spent leaf, but the Stellar transaction envelope still reveals the source
account, the fee payer, the timing, and the destination account that receives
`FIXED_AMOUNT`. That metadata is outside the proof and can be correlated with
off-chain information. The goal of a relayer flow is to remove the
withdrawing account from the transaction envelope and to decouple fee
payment from the withdrawal authorization.

### Options Considered for Soroban

1. **Signer-funded relayer (relayer pays fee and submits)**. The withdrawer
   sends the withdrawal proof and public signals to a relayer out of band.
   The relayer wraps them in a Soroban transaction, pays the fee from its own
   account, and submits the call. This is the simplest option and matches
Stellar's existing fee-bump model.
2. **Fee-bump transaction with the withdrawer as the inner source**. The
   withdrawer signs an inner transaction and a relayer sponsors the fee via
   a fee-bump operation. This keeps the withdrawer as the authorizing account
   but still links the withdrawer's account to the transaction, so it does not
   achieve the goal.
3. **Contract-level fee abstraction**. The pool contract accepts a fee address
   or fee token as part of the withdrawal call and forwards the fee to the
   relayer. This adds contract sturface and new failure modes and is deferred
   until the relayer flow is proven out.
4. **No relayer (current state)**. The withdrawer signs and pays directly.
   Simplest, but leaks the metadata described above.

### Decisions

- **Recipient address.** The recipient address is passed as a public input
  to the withdrawal call and is paid `FIXED_AMOUNT`. It is **not** hidden
  from the ledger; the relayer hides the *sender*, not the *recipient*.
  The recipient address must therefore be treated as public metadata in
  any privacy claim. Binding the recipient into the deposit commitment is
  out of scope for this flow and remains future work.
- **Fee payment.** The relayer pays the Soroban fee from its own account and
  is reimbursed out of band, or through a future contract-level fee address
  field. The fee is **not** paid by the withdrawing account in the relayed
  flow, because that would re-link the withdrawer to the transaction.
- `**Replay protection.** Replay protection remains the existing
  `nullifierHash` double-spend check in the pool contract. The relayer does not
  introduce a new authorization primitive and must not be trusted to enforce
  uniqueness. A malicious relayer can regain nothing by resubmitting a spent
  proof, because the contract rejects already-spent nullifiers. The relayer
  must also not be able to mutate the recipient or amount; those are bound
  into the proof and the contract checks.

### Privacy Gains

- The withdrawing account no longer appears as the transaction source or as
  the fee payer in the ledger envelope.
- Fee payment is decoupled from the withdrawal authorization, so a fee
  payer's account is not linked to the withdrawal in the envelope.
- Timing of the withdrawer's own activity is no longer directly correlated
  with the withdrawal transaction.

### Privacy Limits

- The recipient address and the amount remain public on the ledger. A linked
  off-chain observer can still correlate the payout with the recipient.
- The relayer itself is a metadata chokepoint: if a relayer is the only one
  submitting withdrawals, the relayer's address becomes a correlation key.
  A meaningful privacy claim requires either multiple independent relayers
  or a mixing strategy that is out of scope here.
- The withdrawer must still communicate the proof to the relayer out of band.
  That channel is outside the ZK proof and its metadata must be considered
  separately (e.g. Network addresses, transport logs).
- The contract cannot distinguish a relayed withdrawal from a direct one
  until a fee address field exists. Without that field, relayer reimbursement
  is out-of-band and not verifiable on-chain.
- Relayer liveness and censorship resistance are open problems. A single
  relayer can refuse to submit or delay submission.

### Follow-Up Implementation Tasks

1. Add a relayer entrypoint that accepts the withdrawal proof and public
   signals and submits the contract call from a relayer account.
2. Add a fee address or fee reimbursement field to the withdrawal call and
   test that it cannot be used to redirect the payout.
3. Add tests that a relayed withdrawal cannot be replayed and that a direct
   withdrawal still behaves as before.
4. Document the out-of-band channel and its metadata leaks in this file
   before any privacy claim is made.

---

## Anchor and Off-Ramp Status (Mock)

The **on-ramp (USDC → pool) and off-ramp (pool → INR) corridors are
product-story stubs, not live anchor integrations.**

- Fiat on-ramp and INR off-ramp are not wired to any Stellar anchor
  (SEP-24/SEP-31) or fiat rails. The landing page and dashboard present a
  "Bob INR wallet" corridor as illustrative UI, not as a working corridor.
- The testnet asset is the Stellar **native asset (XLM)** used as a stand-in
  for USDC; there is no USDC, or stablecoin integration on testnet or mainnet.
- There is no deposit-taker, no beneficiary bank account, and no settlement
  guarantee. "1,000 XLM" and "INR" figures in the UI are demo data.
- `Supabase Auth` is wired for real accounts, but public sign-in only works
  once a live Supabase project and deployment environment variables are
  configured; until then the auth forms stay disabled.

Anyone evaluating a corridor should assume **zero** production connectivity
exists today.

---

## Trusted Setup

The Groth16 trusted setup is **not production-secure**:

- The powers-of-tau and circuit-specific ceremony artifacts are generated
  locally by the setup scripts, not by a public ceremony with independent
  participants.
- A compromised toxic waste would allow forging proofs (and therefore forged
  receipts). Do not treat the current setup as a commitment to soundness in
  production.
- Replacing it with a real ceremony is listed as future work.

---

## Legal Caveats

- **Not regulated.** Setu is not a money transmitter, a remittance service, a
  bank, or any kind of licensed financial institution. Nothing in the product
  story, README, or site is an offer to provide regulated services.
- **Not audited.** The codebase has not passed a professional security or
  cryptography audit. The audit mentioned in the README was an internal review
  of a specific cross-layer encoding issue, not a full production audit.
- **Marketing claims are aspirational.** Landing-page statements such as
  "Built for Regulatory Standards" and "meet KYC/AML compliance checks
  on-chain seamlessly" describe a design goal, not a current capability or a
  compliance attestation. See "What the Proof Does Not Prove" above.
- **No legal/compliance/financial advice.** Any use of Setu in a regulated
  context requires qualified legal and compliance review in the relevant
  jurisdictions, and the operator remains responsible for their own
  obligations (KYC/AML/CFT, data protection, licensing).
- **Testnet funds only.** Deployments and transactions are on the Stellar
  testnet with no real value. Nothing in this repository should be used to
  move real funds.

---

## Path To Production

The following gaps would need to close before Setu could responsibly be
considered for anything beyond a prototype:

1. Bind `recipientId`/`purpose` into the deposit commitment (or verify an
   off-ramp signature over them).
2. Replace the local trusted setup with a real, publicly-audited ceremony.
3. Remove the zero-root association bypass from the withdrawal circuit.
4. Integrate real Stellar anchors (SEP-24/SEP-31) and a real off-ramp.
5. Implement and test the relayer flow described above so transaction
   metadata does not leak the sender.
6. Obtain a professional security and cryptography audit.
7. Engage qualified counsel for licensing and KYC/AML/CFT design in each
   operating jurisdiction.

Until then, treat every claim in the product story as a prototype claim, not a
production capability.
