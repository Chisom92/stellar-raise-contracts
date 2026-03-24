# Admin Upgrade Mechanism

## Overview

The `upgrade()` function allows a designated admin to replace the contract's WASM
implementation without changing its address or storage. This enables bug fixes and
feature additions post-deployment.

## How It Works

### Admin assignment

The admin is set once during `initialize()` and stored in instance storage:

```rust
env.storage().instance().set(&DataKey::Admin, &admin);
```

The admin address is separate from the campaign creator — a single trusted party
(e.g. a multisig or governance contract) can manage upgrades across many campaigns.

### `upgrade(new_wasm_hash)`

```rust
pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();
    env.deployer().update_current_contract_wasm(new_wasm_hash);
}
```

- Reads the stored admin address (panics if called before `initialize()`).
- Requires the admin to authorize the call via `require_auth()`.
- Replaces the contract WASM in-place; all storage and the contract address are preserved.

## Security Assumptions

- The admin address should be a multisig or governance contract, not an EOA, to
  prevent a single key compromise from enabling a malicious upgrade.
- `upgrade()` is intentionally not callable before `initialize()` — the `unwrap()`
  on a missing admin key causes a panic, preventing unauthorized upgrades on
  uninitialized contracts.
- The creator has no upgrade authority — admin and creator are distinct roles.
- The WASM hash must be uploaded to the ledger before calling `upgrade()`. An
  invalid or non-existent hash will be rejected by the host environment.

## Test Coverage

File: `contracts/crowdfund/src/admin_upgrade_mechanism_test.rs`

| Test | What it verifies |
|------|-----------------|
| `test_admin_stored_on_initialize` | Admin is stored during `initialize()`; auth check is reached (not a storage panic) |
| `test_non_admin_cannot_upgrade` | A random address is rejected by `upgrade()` |
| `test_creator_cannot_upgrade` | The campaign creator (≠ admin) is rejected by `upgrade()` |
| `test_upgrade_panics_before_initialize` | `upgrade()` panics when no admin is stored |
| `test_upgrade_requires_auth` | Calling `upgrade()` with no auths set is rejected |
| `test_admin_can_upgrade_with_valid_wasm` | Admin succeeds with a real uploaded WASM hash *(ignored: requires release WASM build)* |
