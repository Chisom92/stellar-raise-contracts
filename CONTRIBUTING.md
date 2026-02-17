# Contributing to Stellar Raise Contracts

Thank you for your interest in contributing! 🎉

## How to Contribute

### 1. Fork & Clone

```bash
git clone https://github.com/<your-fork>/stellar-raise-contracts.git
cd stellar-raise-contracts
```

### 2. Create a Branch

```bash
git checkout -b feature/your-feature-name
```

### 3. Make Your Changes

- Write clean, documented Rust code.
- Add or update tests in `test.rs` for any new functionality.
- Run the full test suite before submitting:

```bash
cargo test --workspace
```

### 4. Lint & Format

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

### 5. Open a Pull Request

- Push your branch and open a PR against `main`.
- Provide a clear description of your changes.
- The CI pipeline will automatically run tests and lints.

## Code of Conduct

Be respectful and constructive. We're all here to build something great on Stellar.

## Reporting Issues

Open an issue with a clear title, description, and steps to reproduce. Include your Rust version and OS.
