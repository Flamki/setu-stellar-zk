# Denominations

Status: testnet prototype. Not audited. The design below is implemented in the
contract and CLI and covered by tests, but no real-money deployment is claimed.

## How it works

Every pool has exactly one denomination: the amount, in the pool token's
smallest unit, that each deposit pays in and each withdrawal pays out.

- **Contract.** `__constructor(vk_bytes, token_address, admin, denomination)`
  stores the denomination. It must be greater than zero or deployment fails. It
  cannot be changed afterwards. `get_denomination()` returns it.
- **Withdrawal check.** The circuit only proves the withdrawn value is no more
  than the note value. The contract therefore compares the proof's
  `withdrawnValue` public signal with the denomination and rejects any other
  amount with `Withdrawn value does not match pool denomination`, before any
  payout or nullifier write.
- **CLI.** `stellar-coinutils generate <scope> --denomination <base_units>`
  creates a coin of that value (default `1000000000`). `withdraw` puts the
  coin's own value in the `withdrawnValue` input. The coin value must equal the
  pool's denomination, otherwise the contract rejects the withdrawal.
- **Older pools.** A contract deployed before this change has no stored value
  and reports the default, `1000000000`, so its behaviour is unchanged.

## Units

The denomination is a raw base-unit amount, not a currency amount. Stellar
assets, including the native XLM contract, use 7 decimals, so
`1_000_000_000` base units is 100 units of the asset. Other tokens differ.
Check the token's decimals before choosing a value. Earlier comments calling
the default "1 XLM" were wrong.

## Several denominations

To offer several denominations, deploy one pool per denomination. A pool does
not mix amounts, and nothing in the contract links pools together.

## Privacy-set tradeoffs

A withdrawal is only hidden among the deposits of the **same pool**.

- **Splitting the anonymity set.** Each extra denomination divides the same
  users across more pools. A pool with few deposits gives weak privacy, and a
  rarely used denomination can be close to none.
- **The amount is public.** Deposits and withdrawals show the denomination, so
  the pool a user picks is visible. Choosing a rare denomination marks the
  user out.
- **Combining amounts links them.** Moving 1,000 units as ten deposits of 100
  produces ten withdrawals of 100. Timing, recipient reuse and counts can
  correlate them. Ten withdrawals to one address are easily linked.
- **Large denominations exclude small users.** A high denomination keeps small
  holders out, which shrinks the set further for those who can afford it.
- **Small denominations are noisy and costly.** Paying a large sum takes many
  transactions, each with its own fee and timing signal.
- **Low-volume pools.** On testnet or at launch, pools are small. An observer
  who sees one deposit and one withdrawal can match them.

Practical guidance: prefer few, well-used denominations, avoid unusual values,
and do not describe a pool as private without stating its deposit count.

## Limits

- Denominations are not enforced on the circuit side; the contract enforces them.
- No fees are supported. A relayer fee would need the amount paid out to
  differ from the denomination and a circuit change.
- The UI shows illustrative demo figures, not the denomination of a deployed
  pool.
- Deploying with the wrong denomination is not recoverable: redeploy.
