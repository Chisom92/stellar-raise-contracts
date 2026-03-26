# Contract State Size Limits

The `ContractStateSize` contract defines the boundaries for the crowdfunding platform's persistent storage. It acts as the "source of truth" for the frontend to ensure that all campaign metadata (titles, descriptions, socials, etc.) stays within the platform's optimal performance limits.

## Rationale

Storing data on the Stellar ledger involves costs based on both the number of entries and their byte content (state rent). To maintain economic sustainability and prevent network abuse, we strictly enforce these limits at every contract interaction.

## Features

- **Frontend Configuration**: The UI can query `max_title_length` and `max_description_length` to pre-apply input constraints, reducing transaction reverts.
- **Resource Management**: Provides global caps for contributors, roadmap items, and stretch goals to prevent memory exhaustion in downstream processing.
- **Input Validation**: Reusable validation functions help consistent enforcement across multiple contracts.

## Constants and Functions

### `max_title_length()`
Returns the 128-byte limit for campaign titles.

### `max_description_length()`
Returns the 2,048-byte limit for detailed campaign descriptions.

### `max_contributors()`
Returns the 1,000-contributor cap to ensure batch-processing (e.g., during refunds or NFT minting) stays within gas limits.

### `validate_metadata_aggregate(total_len)`
Calculates whether a proposed set of metadata fields collectively exceeds safe limits, preventing "state rent spikes" from extremely large combined string entries.

## Security Considerations

All limits are verified by their respective contracts (`CrowdfundContract`, `StellarTokenMinter`, etc.) using these queryable helpers to prevent malicious state inflation.
