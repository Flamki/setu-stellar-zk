# Relayer Withdrawal Gas Privacy Design

Status: draft design document. No implementation is claimed here.
This document describes options and decisions only. Any change to code,
tests, or RPE must be landed in its own PR with tests before any privacy
guarantee is asserted.

## Problem

In the current withdrawal flow the recipient signs the transaction and pays the
network fee directly. This links the recipient address to the withdrawal on
chain and leaks metadata that is not covered by the ZK proof:

- The recipient address appears as the source account of the transaction.
- The fee payer is the same address as the recipient.
- Timing and fee amount are visible and correlatable with the deposit.

The ZK proof attests that the withdrawal is authorized by a note in the
tree, but it does not hide who submits the transaction or who pays for it.

## Goals

- Remove the direct link between the recipient address and the transaction
submitter/fee payer.
- Keep the ZK verification and nullifier checks intact.
- Preserve replay protection via the nullifier.
- Document the privacy gains and the remaining limits.

## Non-Goals

- Full anonymity set of any size.
- Hiding the existence of a withdrawal from the contract state.
- Hiding the withdrawal amount from the contract events.
- Replacing the ZK verifier or changing the note format.

## Relayer Options on Soroban

Soroban transactions are submitted by an account that pays the fee and
supplies the source account sequence number. The following options were
considered for decoupling the recipient from the submitter.

### Option A: Recipient-submitted with fee bump

The recipient still signs and submits the withdrawal, but a fee-bump
transaction is added so a second account covers the fee.

- Privacy gain: the fee payer is not the recipient.
- Privacy limit: the recipient address is still the source account of the
withdrawal operation and appears in the transaction envelope.
- Replay protection: unchanged, the nullifier is consumed on chain.
- Effort: low, reuses existing withdrawal path.

### Option B: Relayer submits the withdrawal

A third-party relayer account submits the withdrawal transaction and pays
the fee. The withdrawal call carries the ZK proof, the nullifier, and the
recipient address as a parameter.

- Privacy gain: the source account is the relayer, not the recipient.
- Privacy limit: the recipient address is still visible in the call arguments
unless the contract emits only a commitment and the recipient claims separately.
- Privacy limit: the relayer learns the recipient address and the fee amount
  and can correlate them.
- Replay protection: the nullifier is consumed on chain by the contract.
- Effort: medium, requires a relayer integration and a fee mechanism.

### Option C: Relayer with fee paid from the note

The relayer submits the withdrawal and the contract pays the relayer a
fee deducted from the withdrawal amount. The ZK circuit must attest the fee
and the relayer address so the contract can verify the payment without trusting
the relayer.

- Privacy gain: the recipient does not need a funded account to withdraw.
- Privacy limit: the relayer address and the fee amount are visible on chain.
- Privacy limit: the circuit must be extended to bind the fee and relayer,
which changes the proving key and the verifier.
- Replay protection: the nullifier is consumed on chain and the fee binding
prevents a relayer from replaying the same proof with a different fee.
- Effort: high, requires a circuit change and a new trusted setup.

### Option D: Relayer with a fee vault

The relayer submits the withdrawal and is reimbursed from a fee vault
funded by the user outside the withdrawal flow. This keeps the ZK circuit
unchanged but adds a funding step.

- Privacy gain: the withdrawal call does not carry a fee binding.
- Privacy limit: the vault funding is a separate on-chain transaction that
can be correlated with the recipient.
- Privacy limit: the relayer is paid from a pool that is visible on chain.
- Replay protection: the nullifier is consumed on chain.
- Effort: medium, requires a vault contract and funding flow.

## Decision

The design selects Option B as the first step and Option C as the target.
Option B is the minimal change that removes the recipient from the source
account and from the fee payer. Option C removes the need for the recipient to
hold a funded account and binds the fee in the proof, at the cost of a circuit
change.

### Recipient address

The recipient address is passed as a call argument to the withdrawal
function. The ZK proof binds the note to the withdrawal authority but does not
bind the recipient address in Option B. The contract must not trust the
relayer to choose the recipient; the recipient is part of the call data signed
by the relayer and is verified against the note authority in the circuit when
Option C is enabled.

### Fee payment

In Option B the relayer pays the network fee and is reimbursed by an agreed
amount that is settled outside the contract or by a separate transfer. In
Option C the contract deducts the fee from the withdrawal amount and pays the
relayer. The fee and the relayer address must be binded in the ZK proof so the
contract can reject a proof that was produced for a different fee or relayer.

### Replay Protection

The nullifier is the primary replay protection. The contract must record the
nullifier as spent before releasing funds. The relayer must not be able to replay
the same proof with a different recipient or fee. In Option C the binding of the
fee and relayer in the proof adds a second layer of protection against fee
replay.

## Privacy Gains

- The recipient address is no longer the source account of the transaction.
- The fee payer is no longer the recipient in Option B and Option C.
- The recipient does not need to hold a funded account to withdraw in Option C.

## Privacy Limits

- The withdrawal event is still visible on chain. The existence of a withdrawal
and its timing are not hidden.
- The withdrawal amount is visible in the contract events unless the amount is
split or delayed by a separate mechanism.
- The relayer learns the recipient address in Option B, so the relayer must be
trusted not to correlate it with the deposit.
- The relayer address and fee are visible on chain in Option C.
- A single relayer that serves many withdrawals can be a correlation point.
- The design does not hide the fact that the contract is being used.

## Follow-Up Implementation Tasks

1. Add a relayer entry point to the withdrawal contract that accepts the ZK
proof, the nullifier, and the recipient address as arguments.
2. Add tests that confirm the nullifier is consumed and that a replay with
the same proof is rejected.
3. Extend the ZK circuit to bind the recipient address, and later the fee
and relayer address, and regenerate the verifier and the trusted setup.
4. Add a fee accounting path that deducts the fee from the withdrawal amount
and pays the relayer.
5. Add an integration test that runs the relayer flow end to end on a test
network.
6. Update the README to describe the relayer flow and its limits once the
tests pass.

## Honesty Limits

No implementation is claimed in this document. The design is a prototype
plan. The current code base still uses the recipient-submitted flow. The
privacy properties described here are not guaranteed until the follow-up tasks
are landed with tests. The README and code comments must stay honest about this
status.
