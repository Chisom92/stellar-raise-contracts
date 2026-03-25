# `crowdfund_initialize_function` — Refactored Initialize Logic

## Overview

`crowdfund_initialize_function` extracts and standardizes the `initialize()`
logic from `lib.rs` into a single, auditable module.  It provides:

- A named `InitParams` struct replacing nine positional arguments.
- Pure validation helpers returning typed `ContractError` variants.
- A deterministic, single-pass `execute_initialize()` function with a strict
  checks → effects → storage write ordering.
- An `initialized` event payload for off-chain indexers.
- Helper functions (`describe_init_error`, `is_init_error_retryable`) for
  frontend error handling.

---

## Design Decisions

### Named `InitParams` struct

The original `initialize()` accepted nine positional arguments.  Positional
lists are fragile: swapping two `i128` parameters compiles silently but
produces incorrect on-chain state.  A named struct makes every field explicit
at the call site and lets the compiler catch type mismatches.

### Typed errors instead of panics

The original implementation panicked on invalid platform fee and bonus goal.
Panics are opaque to the frontend — the Soroban SDK surfaces them as a generic
host error with no numeric code.  Typed `ContractError` variants let the
frontend display a specific message without parsing error strings.

| New variant | Code | Trigger |
|---|---|---|
| `InvalidGoal` | 8 | `goal < 1` |
| `InvalidMinContribution` | 9 | `min_contribution < 1` |
| `DeadlineTooSoon` | 10 | `deadline < now + 60` |
| `InvalidPlatformFee` | 11 | `fee_bps > 10_000` |
| `InvalidBonusGoal` | 12 | `bonus_goal <= goal` |

### Validate-before-write ordering

The original code interleaved validation and storage writes.  If a later
validation failed after earlier writes had already committed, the contract
could be left in a partially-initialized state.  `execute_initialize()` runs
all validations first, then writes atomically within the transaction.

### `initialized` event

Soroban storage is not directly queryable by off-chain services without an RPC
call per field.  The `initialized` event carries all campaign parameters in a
single ledger entry, enabling indexers to bootstrap campaign state from the
event stream alone.

---

## Function Reference

### `execute_initialize(env, params) → Result<(), ContractError>`

The single authoritative implementation of campaign initialization.
`CrowdfundContract::initialize()` in `lib.rs` delegates to this function.

**Ordering guarantee:**
1. Re-initialization guard (read-only check on `DataKey::Creator`).
2. `creator.require_auth()` — authentication before any state mutation.
3. Full parameter validation — no storage writes until all checks pass.
4. Storage writes — all-or-nothing within the transaction.
5. Event emission — `("campaign", "initialized")`.

### `validate_init_params(env, params) → Result<(), ContractError>`

Runs all field validations in a single pass.  Delegates to the helpers in
`campaign_goal_minimum` for goal, min_contribution, deadline, and platform fee,
and to `validate_bonus_goal` for the bonus goal ordering constraint.

### `validate_bonus_goal(bonus_goal, goal) → Result<(), ContractError>`

Returns `Ok(())` when `bonus_goal` is `None` or strictly greater than `goal`.
Returns `Err(ContractError::InvalidBonusGoal)` otherwise.

### `describe_init_error(code) → &'static str`

Maps a `ContractError` repr value to a human-readable string.  Intended for
frontend error display.

### `is_init_error_retryable(code) → bool`

Returns `true` for input validation errors (codes 8–12) that the caller can
fix and retry.  Returns `false` for `AlreadyInitialized` (code 1), which is
permanent.

---

## Frontend Interaction

1. Construct the `initialize` transaction with all nine parameters.
2. On success, listen for the `("campaign", "initialized")` event to confirm
   the campaign is live and cache the emitted parameters locally.
3. On failure, read the returned error code and call `describe_init_error(code)`
   to display a user-facing message.
4. Use `is_init_error_retryable(code)` to decide whether to show a "try again"
   button or a permanent failure message.

```typescript
// TypeScript / Stellar SDK example
try {
  await contract.initialize({ admin, creator, token, goal, deadline, ... });
} catch (e) {
  const code = extractContractErrorCode(e); // SDK-specific helper
  const message = describeInitError(code);  // replicate describe_init_error
  const retryable = isInitErrorRetryable(code);
  showError(message, { retryable });
}

// Replicate describe_init_error in TypeScript
function describeInitError(code: number): string {
  const messages: Record<number, string> = {
    1:  "Contract is already initialized",
    8:  "Campaign goal must be at least 1",
    9:  "Minimum contribution must be at least 1",
    10: "Deadline must be at least 60 seconds in the future",
    11: "Platform fee cannot exceed 100% (10,000 bps)",
    12: "Bonus goal must be strictly greater than the primary goal",
  };
  return messages[code] ?? "Unknown initialization error";
}

function isInitErrorRetryable(code: number): boolean {
  return [8, 9, 10, 11, 12].includes(code);
}
```

---

## Scalability Considerations

- `initialize()` is a one-shot function; its gas cost is O(1) regardless of
  future campaign size.
- The `Contributors` and `Roadmap` lists are seeded as empty vectors.  Their
  TTL is managed by `contribute()` and `add_roadmap_item()` respectively.
- The `initialized` event payload is bounded: it contains only scalar values
  and optional scalars, never unbounded collections.
- The `InitParams` struct can be extended with new optional fields in future
  versions without breaking existing callers (new fields default to `None`).

---

## Security Assumptions

1. **Re-initialization guard** — `DataKey::Creator` is used as the
   initialization sentinel.  It is the very first check so no state can be
   written before it.

2. **Creator authentication** — `creator.require_auth()` is called before any
   storage write.  The Soroban host rejects the transaction if the creator's
   signature is absent or invalid.

3. **Goal floor** — `goal >= 1` prevents zero-goal campaigns that could be
   immediately drained by the creator.

4. **Minimum contribution floor** — `min_contribution >= 1` prevents
   zero-amount contributions that waste gas and pollute storage.

5. **Deadline offset** — `deadline >= now + 60s` ensures the campaign is live
   for at least one ledger close interval, preventing dead-on-arrival campaigns.

6. **Platform fee cap** — `fee_bps <= 10_000` ensures the platform can never
   be configured to take more than 100% of raised funds.

7. **Bonus goal ordering** — `bonus_goal > goal` prevents a bonus goal that is
   already met at launch, which would immediately emit a bonus event and confuse
   contributors.

8. **Atomic write ordering** — All validations complete before the first
   `env.storage().instance().set()` call.  A failed validation leaves the
   contract in its pre-initialization state.

---

## Constraints

- `initialize()` can only be called once per contract instance.  The factory
  contract deploys a fresh instance per campaign.
- The `admin` and `creator` may be the same address or different addresses.
  The contract does not enforce a relationship between them.
- `bonus_goal_description` has no length limit enforced at the contract level.
  The frontend should enforce a reasonable display limit (e.g. 280 characters).
- The `initialized` event is emitted after all storage writes.  If the
  transaction is rolled back for any reason, the event is not persisted.

---

## Test Coverage

See [`crowdfund_initialize_function_test.rs`](./crowdfund_initialize_function_test.rs).

Tests cover:

- Normal execution: all fields stored, status Active, empty collections, event emitted
- Platform config: zero fee, exact max fee, fee over max, u32::MAX fee
- Bonus goal: stored with description, equal to goal, less than goal, one above goal, without description
- Re-initialization guard: same params, different params (original values unchanged)
- Goal validation: minimum (1), zero, negative, i128::MIN, i128::MAX
- Min contribution validation: minimum (1), zero, negative
- Deadline validation: exactly 60s, 59s (boundary), equal to now, in past, far future
- `validate_bonus_goal` unit tests: None, greater, equal, less, zero vs one
- `describe_init_error`: all known codes, unknown code fallback
- `is_init_error_retryable`: AlreadyInitialized, all input errors, unknown code
- Integration: contribute, withdraw, get_stats after initialization

Run with:

```bash
cargo test -p crowdfund crowdfund_initialize_function
```
