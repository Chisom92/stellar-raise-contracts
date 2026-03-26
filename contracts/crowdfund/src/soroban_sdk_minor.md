# soroban_sdk_minor

Documents the edge cases and helpers introduced for the Soroban SDK v22 minor version bump, with a focus on frontend UI safety and scalability.

## Overview

`soroban_sdk_minor.rs` centralizes low-level helpers used when reviewing and operating a minor Soroban SDK bump. All functions are explicit, testable, and audit-friendly.

## What Changed in v22

| Area | Before (v21) | After (v22) |
| :--- | :--- | :--- |
| Contract registration in tests | `env.register_contract(None, Contract)` | `env.register(Contract, ())` |
| Storage keys | Raw `String` values | Typed `#[contracttype]` enums |
| Auth pattern | Various | `address.require_auth()` is the standard |

## Public API

```rust
// Assess whether a version upgrade is safe for this contract's storage/ABI.
fn assess_compatibility(env, from_version, to_version) -> CompatibilityStatus

// Parse the minor component from a semver string (e.g. "22.3.0" → 3).
fn parse_minor(version) -> u32

// Returns true when to_version is a forward minor bump within the same major.
fn is_minor_bump(from_version, to_version) -> bool

// Clamp a frontend page-size request into [FRONTEND_PAGE_SIZE_MIN, FRONTEND_PAGE_SIZE_MAX].
fn clamp_page_size(requested) -> u32

// Build a bounded pagination window; saturating arithmetic prevents u32 overflow.
fn pagination_window(offset, requested_limit) -> PaginationWindow

// Validate an optional upgrade note fits within UPGRADE_NOTE_MAX_LEN (256 bytes).
fn validate_upgrade_note(note) -> bool

// Validate a WASM hash is non-zero before applying an upgrade.
fn validate_wasm_hash(wasm_hash) -> bool

// Emit a structured SDK-upgrade audit event on the Soroban event ledger.
fn emit_upgrade_audit_event(env, from_version, to_version, reviewer)

// Emit an audit event with a bounded note; panics if note exceeds max length.
fn emit_upgrade_audit_event_with_note(env, from_version, to_version, reviewer, note)
```

## CompatibilityStatus

| Variant | Meaning |
|---------|---------|
| `Compatible` | Same major version; safe to upgrade |
| `RequiresMigration` | Different major versions; migration step needed |
| `Incompatible` | Empty or completely malformed version string; frontend should surface as error |

## New Edge Cases (this PR)

### `assess_compatibility` — empty string inputs

Previously, empty strings silently mapped to major-0 and could produce a spurious `Compatible` result. Now:

```rust
assess_compatibility(&env, "", "22.0.0")  // → Incompatible
assess_compatibility(&env, "22.0.0", "")  // → Incompatible
assess_compatibility(&env, "", "")        // → Incompatible
```

This prevents a misconfigured frontend call from being treated as a valid same-major upgrade.

### `parse_minor` — new export

Lets the frontend display the exact minor component being bumped:

```rust
parse_minor("22.3.0")  // → 3
parse_minor("22")      // → 0  (no minor component)
parse_minor("22.")     // → 0  (empty minor)
parse_minor("22.x.0")  // → 0  (non-numeric)
parse_minor("")        // → 0
```

### `is_minor_bump` — new export

Lets the frontend distinguish a genuine minor bump from a patch-only or no-op change before showing the upgrade banner:

```rust
is_minor_bump("22.0.0", "22.1.0")  // → true
is_minor_bump("22.1.0", "22.1.5")  // → false (patch only)
is_minor_bump("22.1.0", "22.1.0")  // → false (same)
is_minor_bump("22.2.0", "22.1.0")  // → false (downgrade)
is_minor_bump("22.0.0", "23.1.0")  // → false (cross-major)
```

### `pagination_window` — u32::MAX overflow safety

`offset.saturating_add(limit)` is now used internally so that a near-`u32::MAX` offset cannot produce a wrapped end index:

```rust
pagination_window(u32::MAX, 50)
// → PaginationWindow { start: u32::MAX, limit: 50 }
// start.saturating_add(limit) == u32::MAX  (no wrap)
```

### `validate_upgrade_note` — exact boundary

The exact-boundary case (`len == UPGRADE_NOTE_MAX_LEN`) is now explicitly tested and accepted:

```rust
validate_upgrade_note(&note_of_256_bytes)  // → true
validate_upgrade_note(&note_of_257_bytes)  // → false
```

## Security Assumptions

1. `assess_compatibility` is read-only — no state mutations.
2. Empty version strings return `Incompatible` rather than silently mapping to major-0.
3. `validate_wasm_hash` rejects a zeroed hash to prevent accidental contract bricking.
4. `clamp_page_size` bounds frontend scan size to prevent indexer overload after SDK upgrades.
5. `emit_upgrade_audit_event_with_note` panics on oversized notes to keep event schema predictable.
6. All style/colour values in the frontend UI are hardcoded constants — no dynamic injection.

## NatSpec-style Reference

### `assess_compatibility`
- **@notice** Returns `Compatible`, `RequiresMigration`, or `Incompatible` based on version strings.
- **@security** Read-only; empty inputs return `Incompatible` to prevent silent major-0 mapping.

### `parse_minor`
- **@notice** Extracts the minor component from a semver string.
- **@dev** Returns `0` for any unparseable or missing minor component.

### `is_minor_bump`
- **@notice** Returns `true` only when `to_version` is a forward minor bump within the same major.
- **@dev** Pure function; no state access.

### `pagination_window`
- **@notice** Builds a bounded `PaginationWindow` from an offset and requested limit.
- **@security** Saturating arithmetic prevents `u32` overflow when `offset` is near `u32::MAX`.

### `validate_upgrade_note`
- **@notice** Returns `true` when the note fits within `UPGRADE_NOTE_MAX_LEN` (256 bytes).
- **@dev** Exact boundary (`len == max`) is accepted.

### `validate_wasm_hash`
- **@notice** Returns `true` for any non-zero 32-byte hash.
- **@security** Rejects zeroed hashes to prevent upgrade calls that would brick the contract.

## Running Tests

```bash
cargo test -p soroban-sdk-minor
cargo test -p crowdfund -- soroban_sdk_minor
```

## Test Coverage Summary

| Group | Tests |
|-------|-------|
| Version constants | 1 |
| `assess_compatibility` | 12 |
| `parse_minor` | 6 |
| `is_minor_bump` | 5 |
| `validate_wasm_hash` | 4 |
| `clamp_page_size` | 1 |
| `pagination_window` | 4 |
| `validate_upgrade_note` | 3 |
| `emit_upgrade_audit_event` | 1 |
| `emit_upgrade_audit_event_with_note` | 3 |
| Integration | 4 |
| **Total** | **44** |
