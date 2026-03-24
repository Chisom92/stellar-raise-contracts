# Proptest Generator Boundary Conditions

## NatSpec-Style Documentation

### Overview

This document describes the boundary conditions and constants used by proptest generators for the Stellar Raise crowdfund contract. Correct boundaries ensure property-based tests are stable, secure, and produce data suitable for frontend UI display.

### Typo Fix: Deadline Offset Minimum

**Issue**: The minimum deadline offset was previously documented as **100 seconds**, which:

- Caused proptest regression failures (see `contracts/crowdfund/proptest-regressions/test.txt`)
- Produced flaky tests due to timing races
- Led to poor frontend UX (countdown display flickering for very short campaigns)

**Fix**: The minimum is now **1,000 seconds** (~17 minutes), providing:

- Stable property-based tests
- Meaningful campaign duration for UI display
- Consistent behavior across CI and local runs

---

## Boundary Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `DEADLINE_OFFSET_MIN` | 1,000 | Minimum seconds from `now` to deadline |
| `DEADLINE_OFFSET_MAX` | 1,000,000 | Maximum seconds from `now` to deadline |
| `GOAL_MIN` | 1,000 | Minimum goal (stroops) |
| `GOAL_MAX` | 100,000,000 | Maximum goal for proptest (100M stroops) |
| `MIN_CONTRIBUTION_FLOOR` | 1 | Minimum allowed min_contribution |
| `PROGRESS_BPS_CAP` | 10,000 | Basis points cap (100%) |
| `FEE_BPS_CAP` | 10,000 | Platform fee cap (100%) |

---

## Validation Functions

### `is_valid_deadline_offset(offset: u64) -> bool`

Returns `true` if `offset` is in `[DEADLINE_OFFSET_MIN, DEADLINE_OFFSET_MAX]`.

**Security**: Rejects values that cause timestamp overflow or too-short campaigns.

### `is_valid_goal(goal: i128) -> bool`

Returns `true` if `goal >= GOAL_MIN && goal <= GOAL_MAX`.

**Frontend**: Avoids `goal = 0`, which breaks progress percentage display.

### `is_valid_min_contribution(min_contribution: i128, goal: i128) -> bool`

Returns `true` if `min_contribution` is in `[MIN_CONTRIBUTION_FLOOR, goal]`.

**Contract invariant**: `min_contribution` must not exceed `goal`.

### `is_valid_contribution_amount(amount: i128, min_contribution: i128) -> bool`

Returns `true` if `amount >= min_contribution`.

### `clamp_progress_bps(raw: i128) -> u32`

Clamps raw progress to `[0, PROGRESS_BPS_CAP]`.

**Frontend**: Ensures `progress_bps` never exceeds 100% for display.

---

## Security Assumptions

1. **Overflow**: Goals and contributions are bounded to reduce overflow risk.
2. **Division by zero**: `goal > 0` and `bonus_goal > 0` enforced where division occurs.
3. **Timestamp validity**: Deadline offsets exclude past and unreasonably large values.
4. **Basis points**: `progress_bps` and `fee_bps` capped at 10,000 (100%).

---

## Regression Seeds

The following seeds in `proptest-regressions/test.txt` motivated the boundary fixes:

- `goal = 1_000_000`, `deadline_offset = 100` → Flaky; 100 now rejected.
- `goal = 2_000_000`, `deadline_offset = 100`, `contribution_amount = 100_000` → Same.

---

## Test Coverage

- Unit tests for each constant and validator
- Property tests for valid/invalid ranges
- Edge case tests for regression seeds
- Minimum 95% coverage target

---

## Usage

```rust
use crate::proptest_generator_boundary::{
    DEADLINE_OFFSET_MIN,
    is_valid_deadline_offset,
    is_valid_goal,
};
```

---

## Security Notes

- **Overflow**: Goals and contributions bounded to reduce integer overflow risk.
- **Division by zero**: `goal > 0` and `bonus_goal > 0` enforced where division occurs.
- **Timestamp validity**: Deadline offsets exclude past and unreasonably large values.
- **Basis points**: `progress_bps` and `fee_bps` capped at 10,000 (100%) to prevent display/calculation errors.

## Test Output

Run boundary tests:

```bash
PROPTEST_CASES=100 cargo test -p crowdfund --lib proptest_generator
```

## References

- [Proptest Book](https://altsysrq.github.io/proptest-book/)
- [Soroban Testing](https://soroban.stellar.org/docs/learn/testing)
- Contract: `contracts/crowdfund/src/lib.rs`
