# Relayer Withdrawal Gas Privacy Design

**Status: design only.** This document describes a relayer flow for Setu withdrawals. It is not an implementation claim. No relayer code exists in this repository yet, and nothing here should be read as a working feature. The current contract still requires the recipient to sign and pay for the withdrawal transaction.

---

## Problem

In the current `withdraw(` the recipient address is the transaction source and signer. The ZK proof hides which deposit leaf is being spent, but the Stellar ledger still records the account that submitted the transaction and paid the fee. That account is linked to the withdrawal in time and space, which is a metadata leak outside the proof. Any observer who already knows the recipient's address can confirm that the address withdrew from the pool, even though the proof itself does not reveal the deposit leaf.

## Goal

Break the direct link between the withdrawing account and the withdrawal transaction by having a third-party relayer submit the transaction and pay the fee, while the proof still binds the payout to the recipient's chosen address.

## Non-Goals

- This document does not design a full anonymous payment network. It only addresses gas and transaction-origin metadata.
- It does not claim to hide the fact that a withdrawal occurred, the amount, or the nullifier hash.
- It does not change the current trusted-setup or audit status.

## Soroban Reality Check

Soroban transactions have a source account that signs and pays the fee. There is no native meta-transaction or gas-sponsoring primitive in the contract layer today. Therefore a relayer must be an account that the recipient trusts to submit the transaction on their behalf. The relayer pays the fee and the recipient gets the withdrawal amount. The recipient address is still recorded in the transfer as the destination, so the relayer can link the withdrawal to the recipient if it chooses to. The privacy gain is that the withdrawing account is not the transaction source, not that the recipient is unknowable.

## Relayer Options

### Option A - Single trusted relayer (centralized)

The recipient sends the proof and public signals to a relayer out of band. The relayer submits `withdraw(` with its own account as the transaction source, sets `to` to the recipient address, and pays the fee.

- Privacy gain: the withdrawing account is not the transaction source.
- Limits: the relayer sees the proof and the recipient address, so it can link them. The relayer is a single point of failure and censorship.
- Fee: the relayer pays the Stellar fee in XLM and must be reimbursed out of band.

### Option B - Relayer pool (multiple independent relayers)

Multiple relayers listen for withdrawal requests and compete to submit them. The recipient broadcasts the proof and public signals to a relayer network. Any relayer can submit.

- Privacy gain: the recipient does not have to trust a single relayer. The withdrawing account is not the transaction source.
- Limits: the recipient address is still visible in the transfer, and the relayer that wins the race sees the proof and the recipient address. The relayer pool needs a fee reimbursement mechanism.

### Option C - Proof-bound relayer fee (in-circuit fee)

Extend the withdrawal circuit to include a relayer fee and a relayer address as public inputs. The contract transfers the fee to the relayer and the remainder to the recipient.

- Privacy gain: the fee is paid from the pool, so the relayer does not need to be reimbursed out of band. The relayer address is binded into the proof, so a relayer cannot steal the fee.
- Limits: this is a circuit and contract change, not a deployment configuration. It requires a new trusted setup for the modified circuit. The recipient address is still visible in the transfer.

### Option D - Native gas-sponsoring (future Soroban feature)

If Soroban ever exposes a native fee-sponsoring or meta-transaction primitive, the recipient could sign the withdrawal without paying the fee. This is not available today and is out of scope for the immediate relayer work.

## Decision

For the first implementation, use **Option A** (a single trusted relayer) with a clear documented trust model. It is the smallest change that actually breaks the direct link between the withdrawing account and the transaction. Option B can follow once the fee reimbursement flow is proven. Option C is the long-term goal but requires a circuit change and a new trusted setup, so it is out of scope for the first PRs.

## Recipient Address

The recipient address is not hidden by the ZK proof. The `withdraw()` call already takes `to: Address` and the contract transfers the fixed amount to that address. The relayer flow does not change this. The recipient address is still visible in the transfer event and in the transaction result. What changes is that the recipient address is no longer the transaction source or the fee payer.

To keep the recipient address from being linked to the withdrawing account, the relayer must not be the same account as the recipient, and the relayer must not be a deterministic function of the recipient address. The design document recommends the relayer use a fresh account per withdrawal or a common account that is not derived from the recipient.

## Fee Payment

The relayer pays the Stellar network fee in XLM. The fee is paid from the relayer's account, not from the pool. The recipient receives the full `FIXED_AMOUNT` because the contract transfers exactly `FIXED_AMOUNT` to `to`. The relayer must be reimbursed out of band for the first implementation.

The fee reimbursement model is documented here but not implemented:

- The recipient and relayer agree on a fee amount out of band.
- The relayer submits the withdrawal and pays the network fee.
- The recipient pays the relayer the agreed fee out of band, either before or after the withdrawal settles.

This is a trust assumption. If the recipient does not pay, the relayer loses the fee. If the relayer does not submit, the recipient loses nothing but time. The design document does not claim to solve this trust problem.

## Replay Protection

Replay protection is already enforced by the contract through the nullifier mechanism:

- The withdrawal proof reveals `nullifierHash` as a public signal.
- The contract checks whether `nullifierHash` has been used before (`Error.NullifierUsed`).
- On success, the contract adds `nullifierHash` to the used-nullifier list.

Because the nullifier is bound to the note and the contract records it on the first successful withdrawal, a relayer cannot replay the same proof to withdraw twice. The relayer flow does not need any additional replay protection. The existing nullifier check is the only replay guard, and it is already tested in the contract test suite.

## Privacy Gains and Limits

### Gains

- The withdrawing account is not the transaction source, so the account that knows the note opening is not on-chain link-able to the withdrawal by default.
- The fee is paid by the relayer, so the recipient account does not need to hold XLM or appear in the transaction fee history.
- The nullifier hash is still the only public link to the spent note, and it is already part of the proof's public signals.

### Limits

- The recipient address is still visible in the transfer. The relayer flow hides the withdrawing account, not the recipient.
- The relayer sees the proof and the recipient address, so the relayer can link them if it chooses to. This is a trust assumption, not a cryptographic guarantee.
- The fact that a withdrawal occurred, the amount, and the nullifier hash are still public.
- Timing correlation remains possible. If the recipient deposits and withdraws in a predictable pattern, an observer may still infer links.
- There is no guarantee that the relayer will submit the transaction. The recipient depends on the relayer's good behavior.
- The design does not hide the relayer's own account activity. If the relayer is unique to one withdrawal, the relayer account itself becomes a correlation point.

## Follow-Up Implementation Tasks

1. Add a relayer client that accepts a withdrawal proof and public signals, submits `withdraw()` from a relayer account, and pays the fee.
2. Add a relayer request format and an out-of-band fee agreement checklist.
3. Add a test that shows the withdrawing account is not the transaction source when the relayer submits.
4. Add a test that shows a replay of the same proof fails with `NullifierUsed`.
5. Extend the circuit and contract to bind a relayer fee and relayer address into the proof (Option C).
6. Replace the local trusted setup with a public ceremony before any relayer fee feature is claimed to be secure.

## Honesty Notes

- This document is a design proposal. No relayer code has been written or tested.
- The current contract still requires the recipient to sign and pay for the withdrawal. The relayer flow is future work.
- The privacy gain from a relayer is limited to transaction-origin metadata. It does not make the withdrawal anonymous.
- The relayer trust model is not solved by this design. Any relayer implementation must be evaluated for censorship resistance and fee reimbursement risk.
- This document does not claim that the relayer flow is implemented, tested, audited, or production-ready.
