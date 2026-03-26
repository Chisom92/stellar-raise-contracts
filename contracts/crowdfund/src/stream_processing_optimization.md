# Stream Processing Optimization

## Overview

`stream_processing_optimization.rs` introduces a small set of helpers that tighten up the contract's most common address-stream and aggregate-processing paths:

- contributor membership checks are loaded once and reused
- contributor and pledger streams are only persisted when a new address is actually appended
- campaign aggregate stats are built through a single bounded scan
- stretch-goal and bonus-goal progress use shared progress helpers instead of duplicating arithmetic

The goal is to improve gas efficiency without changing business logic or making review harder.

## What Changed

### `load_address_stream_state`

Loads a stored address vector once and caches whether a target address is already present.

Why it helps:

- removes duplicate storage reads in `contribute`
- removes duplicate storage reads in `pledge`
- keeps membership logic explicit and easy to audit

### `persist_address_stream_if_missing`

Appends and persists a new address only when it is not already present.

Why it helps:

- preserves set-like semantics for contributor and pledger tracking
- prevents duplicate entries from inflating downstream scans
- centralizes TTL extension with the write path

### `compute_progress_bps` and `bonus_goal_progress_bps`

Shared progress helpers now provide:

- zero-goal guards
- negative-value guards
- saturating multiplication before basis-point scaling
- a hard cap at `10_000` bps

### `collect_contribution_stats` and `build_campaign_stats`

Campaign stats are now built through a bounded scan that:

- computes contributor count, average contribution, and largest contribution in one pass
- fails closed if the contributor stream exceeds the configured scan cap
- reuses the stored `total_raised` value instead of re-summing the stream

### `next_unmet_milestone`

Encapsulates the ordered stretch-goal scan so milestone selection uses a single, reusable helper.

## Integration Points

The main crowdfund contract now uses the new module in:

- `contribute`
- `pledge`
- `current_milestone`
- `bonus_goal_progress_bps`
- `get_stats`

## Security Notes

### Address-stream integrity

`persist_address_stream_if_missing` keeps contributor and pledger vectors deduplicated. This matters because duplicate addresses could distort aggregate views and increase per-call processing costs.

### Bounded aggregate scans

`collect_contribution_stats` asserts that the contributor stream does not exceed `MAX_STREAM_SCAN_ITEMS`, which is aligned with the contract's contributor cap. That means unexpected state growth fails closed instead of silently allowing an unbounded scan.

### Arithmetic safety

Progress calculations use saturating multiplication before scaling to basis points. This avoids overflow when very large balances are processed.

### Data-flow assumptions

The helper module assumes:

- contributor and pledger vectors are the canonical ordered address streams
- per-address contribution values remain stored under `DataKey::Contribution(Address)`
- the existing contributor cap is enforced at write time

If those assumptions change later, the helper module should be updated alongside the storage schema.

## Tests

`stream_processing_optimization.test.rs` covers:

- progress computation edge cases
- bonus-goal progress handling
- first-unmet-milestone selection
- cached membership loading
- duplicate-safe persistence
- bounded aggregate scan behavior
- composed campaign-stat generation

## Review Notes

This change is intentionally narrow:

- no new external entrypoints
- no authorization changes
- no storage-schema migration
- only shared helper logic plus targeted call-site integration
