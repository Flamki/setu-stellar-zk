# Relayer Withdrawal Gas Privacy Design

**Status: design only.** No relayer is implemented in this repository. This document describes options, makes a recommendation, and lists follow-up work. Every claim below is a design hypothesis until the corresponding implementation task lands with tests.

---

## Problem

In the current withdrawal flow (`contract/src/lib.rs`, `withdraw`), the recipient calls the contract directly and pays the Soroban network fee from their own account. The ZK proof hides *which deposit* is being spent, but it does not hide the withdrawing account. The transaction envelope publishes:

- The source account that signed and paid the fee.
- The timing of the withdrawal.
- The fact that this account is tied to the recipient address in the transfer.

This is the gap the relayer flow is addressing. The ZK proof already proves authorization to spend a note; the recipient does not need to be the transaction submitter or the fee payer.

---

## Privacy Gains

A deployed relayer would move the following metadata out of the public transaction envelope:

1. **Fee payer decoupling.** The relayer pays the network fee and submits the transaction. The recipient address is no longer the source account in the transaction envelope.
2. **Timing decoupling.** The recipient no longer needs to submit a transaction at the moment of withdrawal. The relayer can batch or delay submission, weakening time-based correlation between deposit and withdrawal.
3. **Address decoupling.** With a relayer, the recipient address can be a fresh address that has never paid fees, never held a balance, and never appeared in any other transaction. The recipient address still appears in the transfer, but it is no longer linked to a signing account.

---

## Privacy Limits

A deployed relayer does **not** solve the following. These are the limits that must be stated honestly in the product story and in the compliance document.

1. **The recipient address is still public.** The contract transfers `FIXED_AMOUNT` to a specific address. That address is visible on chain. A relayer hides *who paid the fee**, not *who received the funds**.
2. **The amount is still public.** The contract only supports one fixed denomination. A linked sequence of withdrawals to the same recipient is visible as a sequence of equal-size transfers.
3. **The nullifier hash is still public.** The withdrawal proof's public signals include `nullifierHash`, and the contract stores it in the used-nullifier list. The relayer does not hide this.
4. **The relayer itself is a trust and metadata center.** The relayer sees the recipient address, the proof, and the timing before submission. It can censor, delay, or correlate withdrawals. A single relayer is a single point of metadata leakage.
5. **Network-level correlation remains.** If the recipient connects to the relayer from an IP address that is also used for other identifying activity, the relayer can link the withdrawal to that IP. This is outside the scope of the contract and the proof.
6. **The relayer must be funded.** The relayer pays fees in XLM. The recipient must compensate the relayer in a way that does not reintroduce a public link between the recipient and the withdrawal. This is the hardest part of the design (see below).

---

## Relayer Options for Soroban Platform

### Option A - Out-of-band relayer with off-chain fee compensation

The recipient contacts a relayer out of band (e.g. an HTTP endpoint or a messaging channel), sends the proof and public signals, and the relayer submits the transaction. The relayer is compensated off-chain (e.g. a separate payment, a credit line, or a subscription).

- **Pros:** Simplest to build. No contract change is required. The existing `withdraw` signature can be reused if the relayer is given the recipient's authorization (see below).
- **Cons:** The recipient must trust the relayer to submit. The relayer can censor or delay. The recipient must pay the relayer in a way that is traceable or requires an account that links to them.
- **Soroban fit:** Works today with the existing contract if the authorization model below is used.

### Option B - On-chain relayer with fee compensation from the withdrawal

A contract change adds a relayer fee that is deducted from the withdrawal amount and paid to the transaction submitter. The recipient signs a relayer authorization that specifies the relayer address and the fee amount.

- **Pros:** The recipient does not need to hold or pay XLM for fees. The relayer is paid from the withdrawal itself. The fee is bound to the proof and to the relayer address.
- **Cons:** Requires a contract change and a new circuit public signal for the fee amount and relayer address. The fee amount is public, which leaks information about the relayer's economics. The fixed denomination is no longer exactly fixed from the recipient's perspective.
- **Soroban fit:** Soroban contracts can transfer to two destinations in one invocation, so this is feasible. The circuit change is the main cost.

### Option C - Meta-transaction with a fee sponsor (Stellar fee-bump)

Stellar supports fee-bump transactions, where a sponsor account pays the fee for a transaction signed by another account. The recipient signs the transaction and the sponsor pays the fee.

- **Pros:** No contract change. The recipient's authorization is preserved because they still sign the transaction. The fee payer is decoupled from the recipient.
- **Cons:** The recipient still signs the transaction, so their address is still in the transaction envelope as a signer. This is not a full decoupling. It hides the fee payer, not the signer.
- **Soroban fit:** Stellar fee-bump is a native protocol feature. This is the lowest-effort option that actually moves fee payment off the recipient.

### Option D - Relayer network with multiple independent relayers

A set of independent relayers compete to submit withdrawals. The recipient broadcasts the proof to the network, and a relayer picks it up and submits it.

- **Pros:** No single relayer sees all metadata. Censorship resistance is higher.
- **Cons:** Much more complex. Requires a broadcast channel, a fee market, and a way to prevent double-submission of the same proof. The nullifier already prevents double-spending on-chain, but not double-submission effort.
- **Soroban fit:** Soroban has no native mempool broadcast for this use case. This would require an off-chain coordination layer.

---

## Recommendation

For the next iteration, implement **Option C** first because it requires no contract change and no circuit change, and it already moves the fee payer out of the recipient's account. Then evaluate **Option A** for a full recipient-signer decoupling. Option B is the most complete but requires a circuit change and should be treated as a separate work item. Option D is out of scope for the next iteration.

---

## Decisions: Recipient Address, Fee Payment, Replay Protection

### Recipient Address

The recipient address is a constructor parameter of the withdrawal transaction. It is not part of the ZK proof's public signals. The contract transfers `FIXED_AMOUNT` to whatever address the caller passes as `to`.

In a relayer flow, the recipient address is chosen by the recipient and communicated to the relayer out of band. The relayer includes it in the transaction. The recipient address is still public on chain.

**Decision:** The recipient address remains a transaction parameter, not a proof signal. This is consistent with the current contract and avoids a circuit change. The privacy gain from the relayer is that the recipient address is not the fee payer or the signer.

### Fee Payment

**Decision:** The relayer pays the network fee from its own account. The relayer is compensated out of band by the recipient. This is Option A. The contract does not need to know about the relayer or the fee.

This decision is deliberately conservative: it requires no contract change, no circuit change, and no new public signal. The tradeoff is that the recipient must pay the relayer in a way that does not reintroduce a public link. This is an operational and business question, not a cryptographic one.

If the recipient must pay in XLM from an account that is already linked to them, the privacy gain is lost. This must be stated honestly in the product story.

### Replay Protection

Replay protection already exists in the contract via the nullifier mechanism:

- The withdrawal proof includes `nullifierHash` as a public signal.
- The contract checks that `nullifierHash` has not been used before (`ErrorNullifierUsed`).
- The contract adds the nullifier to the used-nullifier list after a successful withdrawal.

**Decision:** The existing nullifier mechanism is sufficient for replay protection in the relayer flow. No additional replay protection is needed. The relayer cannot replay a withdrawal because the nullifier is already spent.

The relayer can, however, fail to submit a transaction after receiving it. This is a censorship problem, not a replay problem. The recipient can retry with a different relayer because the nullifier is not yet spent.

---

## Authorization Model

The current `withdraw` function calls `to.require_auth()`, which means the recipient must sign the transaction. In a relayer flow, the relayer submits the transaction, so the recipient is not the signer.

This is the key obstacle for any relayer flow. There are two ways to resolve it:

1. **Signed authorization (current contract, no change):** The recipient signs an authorization for the relayer to submit on their behalf. On Stellar, this can be a fee-bump transaction where the recipient signs the transaction and the relayer pays the fee. This is Option C. The recipient's address is still in the envelope as a signer, but the fee payer is decoupled.

2. **Proof-based authorization (contract change):** Remove `to.require_auth()` and rely on the ZK proof as the authorization. The proof already proves knowledge of the note opening. The contract would need to bind the recipient address into the proof or into an authorization signature to prevent a relayer from redirecting the funds. This is a contract and circuit change.

**Decision:** For the next iteration, use **signed authorization (fee-bump)**. It requires no contract change and no circuit change. The full proof-based authorization is a follow-up work item.

---

## Follow-Up Implementation Tasks

1. **Relayer client (cli)** - Add a `cli/relayer` crate that takes a proof and public signals, builds a fee-bump transaction with the recipient as the transaction signer, and submits it to Soroban. Test on testnet with a mock recipient account.

2. **Recipient authorization flow** - Document and implement the client-side flow where the recipient signs the transaction and the relayer pays the fee. Verify that the contract accepts the relayed-signed transaction.

3. **Relayer fee accounting** - Define how the relayer is compensated out of band and how this is audited. Document the tradeoff in the product story.

4. **Censorship resistance** - Evaluate whether a multi-relayer setup is warranted for the next iteration. Document the tradeoffs.

5. **Privacy tests** - Add integration tests that confirm the relayer flow does not leak the recipient address as the fee payer. Verify that the nullifier still prevents replay.

6. **Documentation update** - Update `docs/privacy-compliance-limitations.md` to reflect the relayer flow once it is implemented and tested. Remove the "No gas/network-metadata privacy" caveat only after the relayer is working.

---

## Honesty Checklist

Before claiming any privacy gain from the relayer flow, the following must be true:

- [ ] A relayer client exists and is tested on testnet.
- [ ] The recipient address is not the fee payer in any test transaction.
- [ ] The nullifier still prevents replay in the relayer flow.
- [ ] `docs/privacy-compliance-limitations.md` is updated to reflect the new flow.
- [ ] No claim is made about hiding the recipient address or the amount.

---

## Summary

| Question | Decision |
| --- | --- |
| Who submits the transaction? | Relayer |
| Who pays the network fee? | Relayer |
| How is the relayer compensated? | Out of band by the recipient |
| How is the recipient authorized? | Signed authorization (fee-bump) |
| How is replay prevented? | Existing nullifier mechanism |
| What is hidden? | The fee payer and the signer of the fee payment |
| What is still public? | Recipient address, amount, nullifier hash, timing |
| What is not solved? | Network-level correlation, relayer trust, amount linkability |
