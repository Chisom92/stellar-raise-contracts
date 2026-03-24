# Bounded `withdraw()` Event Emission

## Overview

This change caps the number of NFT mint calls and their associated events emitted
inside a single `withdraw()` invocation, preventing unbounded gas consumption when
a campaign has a large contributor list.

## Problem

The original `withdraw()` loop iterated over every contributor and emitted one
`nft_minted` event per contributor:

```rust
for contributor in contributors.iter() {
    // ... mint NFT ...
    env.events().publish(("campaign", "nft_minted"), (contributor, token_id));
}
```

With no upper bound, a campaign with thousands of contributors would emit thousands
of events in a single transaction, causing unpredictable and potentially excessive
resource consumption.

## Solution

### Constant: `MAX_NFT_MINT_BATCH`

```rust
pub const MAX_NFT_MINT_BATCH: u32 = 50;
```

Defined in `lib.rs`. Controls the maximum number of NFT mints (and their events)
per `withdraw()` call. Adjust this value based on network resource limits.

### Changes to `withdraw()`

1. The NFT loop now breaks after `MAX_NFT_MINT_BATCH` eligible contributors.
2. Per-contributor `nft_minted` events are replaced with a **single summary event**:

```rust
env.events().publish(("campaign", "nft_batch_minted"), minted_count);
```

3. The `withdrawn` event now carries a third field — the number of NFTs minted:

```rust
env.events().publish(("campaign", "withdrawn"), (creator, payout, nft_minted_count));
```

## Events Reference

| Topic 1    | Topic 2             | Data                              | When emitted                        |
|------------|---------------------|-----------------------------------|-------------------------------------|
| `campaign` | `withdrawn`         | `(Address, i128, u32)`            | Always on successful withdraw       |
| `campaign` | `nft_batch_minted`  | `u32` (count minted)              | Only when NFT contract is set and ≥1 contributor was minted |
| `campaign` | `fee_transferred`   | `(Address, i128)`                 | Only when platform fee is configured |

## Security Assumptions

- The cap does **not** skip contributors permanently — it only limits a single
  `withdraw()` call. If batch minting for all contributors is required, the NFT
  contract owner should implement a separate claim mechanism.
- `MAX_NFT_MINT_BATCH` is a compile-time constant. Changing it requires a contract
  upgrade via `upgrade()` (admin-only).
- The `withdrawn` event data change (added `nft_minted_count`) is a breaking change
  for off-chain indexers that decoded the old two-field tuple. Indexers must be
  updated to handle the new three-field tuple `(Address, i128, u32)`.

## Test Coverage

File: `contracts/crowdfund/src/withdraw_event_emission_test.rs`

| Test | What it verifies |
|------|-----------------|
| `test_withdraw_mints_all_when_within_cap` | All contributors minted when count < cap |
| `test_withdraw_caps_minting_at_max_batch` | Only `MAX_NFT_MINT_BATCH` minted when count > cap |
| `test_withdraw_mints_exactly_at_cap_boundary` | Exact boundary: count == cap mints exactly cap |
| `test_withdraw_emits_single_batch_event` | Exactly one `nft_batch_minted` event emitted |
| `test_withdraw_no_batch_event_without_nft_contract` | No batch event when NFT contract not set |
| `test_withdraw_emits_withdrawn_event_once` | `withdrawn` event emitted exactly once |
| `test_withdraw_no_batch_event_when_no_eligible_contributors` | Batch event still fires for ≥1 contributor |
