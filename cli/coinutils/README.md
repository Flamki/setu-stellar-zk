# CoinUtils - Privacy Pool Coin Utilities

A modular Rust application for managing privacy pool coins, including generation, withdrawal, and association set management.

## Features

- `Coin Generation`: Create new privacy pool coins with cryptographic commitments
- `Coin Withdrawal`: Generate SNARK inputs for coin withdrawals with merkle proofs
- `Association Set Management`: Manage association sets for privacy pool operations
- `Modular Architecture`: Clean separation of concerns with well-defined modules
- `Comprehensive Testing`: Unit tests and integration tests
- `Logging Support`: Configurable logging with different levels

## Architecture

The application is organized into the following modules:

### Core Modules

- `types/`: Data structures for coins, state files, and SNARK inputs
- `crypto/`: Cryptographic operations including Poseidon hashing and conversions
- `merkle/`: Merkle tree operations for withdrawals and association sets
- `io/`: File I/O operations and serialization
- `cli/`: Command-line interface and argument parsing
- `error/`: Custom error types and error handling

### Configuration

- `config.rs`: Application constants and configuration values

## Usage

### Generate a Coin

```bash
stellar-coinutils generate my_pool_scope coin.json
```

### Withdraw a Coin

```bash
stellar-coinutils withdraw coin.json state.json association.json withdrawal.json
```

### Update Association Set

```bash
stellar-coinutils updateAssociation association.json "1234567890..."
```

## File Formats

### Coin File Format

```json
{
  "coin": {
    "value": "1000000000",
    "nullifier": "...",
    "secret": "...",
    "label": "...",
    "commitment": "..."
  },
  "commitment_hex": "0x..."
}
```

### State File Format

```json
{
  "commitments": ["commitment1", "commitment2", ...],
  "scope": "pool_scope"
}
```

### Association Set File Format

```json
{
  "labels": ["label1", "label2", "label3", "label4"],
  "scope": "pool_scope",
  "root": "merkle_tree_root"
}
```

## Withdrawal Relayer Flow (Design)

> Status: design only. The current code in this crate generates withdrawal SNARK inputs and does not yet implement a relayer. Nothing in this section is a claim that a relayer has been built or tested.

### Motivation

In the current flow, the recipient signs the withdrawal transaction and pays the network fee directly. The ZK proof hides the coin's origin, but the account that submits and pays for the transaction is linked on-chain to the withdrawal event. This leaks metadata outside the proof: an observer can associate the withdrawal address with the fee payer, and the fee payer must be funded from somewhere.

### Relayer Options on Soroban

Soroban executes contracts deterministically from a signed transaction; there is no on-chain mempool of intents that a contract can inspect. Any relayer design therefore has to work with the transaction authorization model. Two viable options:

1. **Relayer-submitted transaction with fee reimbursement.** The recipient constructs the withdrawal call and authorizes it without signing a fee-paying transaction. A relayer submits the transaction and pays the fee, then the pool pays the relayer a fee from the withdrawn value. This is the closest match to the classic privacy-pool relayer model and keeps the recipient address out of the fee-paying role.

2. **Fee-bumped or sponsored transaction.** The recipient still signs the withdrawal, but a separate account covers the fee. This is simpler to build on Soroban but only helps if the recipient address is not otherwise linked to the submitting account. It is a weaker privacy guarantee because the recipient's signature and address are still on the transaction.

The recommended direction is option 1, with option 2 as an intermediate step if the fee reimbursement logic is not yet in the contract.

### Recipient Address

The withdrawal call takes a recipient address as an input. The ZK proof attests that the coin is valid and unspent, but the recipient address is not hidden by the proof in the current design. To keep the recipient out of the fee-paying role, the recipient address must be binded into the proof (for example as a public input or hashed into a public input) so the relayer cannot substitute its won address. Without this binding, a relayer could redirect the withdrawal.

### Fee Payment

The fee is paid from the withdrawn value to the relayer's address. The fee amount must be bound into the proof or otherwise committed to by the recipient, otherwise a relayer could extract an arbitrary fee. The contract should enforce that the fee is within an acceptable range and that the remaining value goes to the recipient.

### Replay Protection

Replay protection is already provided by the nullifier in the ZK proof: each coin can be spent once. The relayer flow does not change this. However, the relayer must not be able to replay a valid withdrawal authorization with a different recipient or fee, which is why the recipient and fee must be bound into the proof or the authorization.

### Privacy Gains

- The recipient address is no longer the fee-paying account, so the withdrawal event is not directly linked to a funded account that the recipient controls.
- The relayer provides a separate on-chain identity from the recipient, breaking the simple fee-payer link.

### Privacy Limits

- The relayer itself is visible on-chain and can correlate withdrawals it submits. A single relayer serving many users can become a metadata hub.
- Timing and amount correlation still exists. The ZK proof hides the coin's origin, but not the fact that a withdrawal of a particular value occurred at a particular time.
- If the recipient address is not binded into the proof, a relayer can redirect the withdrawal.
- The fee amount is public and can be used to cluster withdrawals.

### Follow-up Implementation Tasks

- [] Bind the recipient address into the withdrawal proof or authorization.
- [] Add fee reimbursement logic to the pool contract so a relayer can be paid from the withdrawn value.
- [] Add a relayer cli command that submits a withdrawal transaction and collects the fee.
- [] Add integration tests covering the relayer flow and replay protection.
- [] Document the relayer API and how recipients authorize withdrawals.

## Development

### Running Tests

```bash
cargo test
```

### Running Integration Tests

```bash
cargo test --test integration_tests
```

### Building

```bash
cargo build --release
```

## Logging

The application supports configurable logging. Set the `RUST_LOG` environment variable to control log levels:

```bash
RUST_LOG=debug stellar-coinutils generate my_scope coin.json
```

Available log levels: `error`, `warn`, `info`, `debug`, `trace`

## Dependencies

- `Soroban SDK`: Stellar smart contract development
- `Poseidon`: Cryptographic hash function
- `Lean IMT`: Incremental Merkle Tree implementation
- `Clap`: Command-line argument parsing
- `Serde`: Serialization/deserialization
- `ThisError`: Error handling
- `Log/Env Logger`: Logging support

## Error Handling

The application uses a custom error type hierarchy with proper error propagation and user-friendly error messages. All operations return `Result<T, CoinUtilsError>` for consistent error handling.

## Testing

The project includes:

- `Unit Tests`: In each module for testing individual components
- `Integration Tests`: End-to-end testing of complete workflows
- `Test Data`: Sample files for testing different scenarios
