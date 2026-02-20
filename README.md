# 🚀 Stellar Raise Contracts

A **crowdfunding smart contract** built on the [Stellar](https://stellar.org/) network using [Soroban](https://soroban.stellar.org/).

## Overview

Stellar Raise lets anyone create a crowdfunding campaign on-chain. Contributors pledge tokens toward a goal before a deadline. If the goal is met, the creator withdraws the funds. If not, contributors are refunded automatically.

### Key Features

| Feature | Description |
|---|---|
| **Initialize** | Create a campaign with a goal, deadline, and token |
| **Contribute** | Pledge tokens before the deadline |
| **Withdraw** | Creator claims funds after a successful campaign |
| **Refund** | Contributors reclaim tokens if the goal is missed |

## Project Structure

```
stellar-raise-contracts/
├── .github/workflows/rust_ci.yml   # CI pipeline
├── contracts/crowdfund/
│   ├── src/
│   │   ├── lib.rs                  # Smart contract logic
│   │   └── test.rs                 # Unit tests
│   └── Cargo.toml                  # Contract dependencies
├── Cargo.toml                      # Workspace config
├── CONTRIBUTING.md
├── README.md
└── LICENSE
```

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- The `wasm32-unknown-unknown` target:

  ```bash
  rustup target add wasm32-unknown-unknown
  ```

- [Stellar CLI](https://soroban.stellar.org/docs/getting-started/setup) (optional, for deployment)

## Getting Started

```bash
# Clone the repo
git clone https://github.com/<your-org>/stellar-raise-contracts.git
cd stellar-raise-contracts

# Build the contract
cargo build --release --target wasm32-unknown-unknown

# Run tests
cargo test --workspace
```

## Contract Interface

```rust
// Create a new campaign
fn initialize(env, creator, token, goal, deadline);

// Pledge tokens to the campaign
fn contribute(env, contributor, amount);

// Creator withdraws after successful campaign
fn withdraw(env);

// Refund all contributors if goal not met
fn refund(env);

// View functions
fn total_raised(env) -> i128;
fn goal(env) -> i128;
fn deadline(env) -> u64;
fn contribution(env, contributor) -> i128;
```

## Deployment (Testnet)

```bash
# Build the optimized WASM
cargo build --release --target wasm32-unknown-unknown

# Deploy using Stellar CLI
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/crowdfund.wasm \
  --network testnet \
  --source <YOUR_SECRET_KEY>
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a full history of notable changes.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.
