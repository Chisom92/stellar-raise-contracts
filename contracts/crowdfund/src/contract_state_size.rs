//! Contract State Size Limits
//!
//! This contract defines and exposes the maximum size limits for all campaign-related data
//! stored in the Stellar Raise platform's on-chain state. These limits ensure:
//!
//! - **Resource Efficiency**: Prevents ledger state bloat by capping entry sizes.
//! - **Frontend Reliability**: The UI can validate inputs locally against these known limits.
//! - **Predictable Fees**: State-rent (storage) costs remain within predictable bounds.
//!
//! All constants are measured in bytes (for strings) or item counts (for vectors).

#![no_std]

use soroban_sdk::{contract, contractimpl, Env, String};

// ── State Limits ─────────────────────────────────────────────────────────────

/// Maximum campaign title length in bytes.
pub const MAX_TITLE_LENGTH: u32 = 128;

/// Maximum campaign description length in bytes.
pub const MAX_DESCRIPTION_LENGTH: u32 = 2_048;

/// Maximum social-links string length in bytes.
pub const MAX_SOCIAL_LINKS_LENGTH: u32 = 512;

/// Maximum number of unique contributors tracked per campaign.
pub const MAX_CONTRIBUTORS: u32 = 1_000;

/// Maximum number of roadmap items stored for a campaign.
pub const MAX_ROADMAP_ITEMS: u32 = 32;

/// Maximum number of stretch goals (milestones).
pub const MAX_STRETCH_GOALS: u32 = 32;

/// Minimum allowed campaign goal in token units.
pub const MIN_GOAL_AMOUNT: i128 = 100;

#[contract]
pub struct ContractStateSize;

#[contractimpl]
impl ContractStateSize {
    /// Returns the maximum allowed title length in bytes.
    /// @dev Used by frontend UI to set input field `maxlength`.
    pub fn max_title_length(_env: Env) -> u32 {
        MAX_TITLE_LENGTH
    }

    /// Returns the maximum allowed description length in bytes.
    pub fn max_description_length(_env: Env) -> u32 {
        MAX_DESCRIPTION_LENGTH
    }

    /// Returns the maximum allowed social links length in bytes.
    pub fn max_social_links_length(_env: Env) -> u32 {
        MAX_SOCIAL_LINKS_LENGTH
    }

    /// Returns the maximum number of contributors per campaign.
    pub fn max_contributors(_env: Env) -> u32 {
        MAX_CONTRIBUTORS
    }

    /// Returns the maximum number of roadmap items.
    pub fn max_roadmap_items(_env: Env) -> u32 {
        MAX_ROADMAP_ITEMS
    }

    /// Returns the maximum number of stretch goals.
    pub fn max_stretch_goals(_env: Env) -> u32 {
        MAX_STRETCH_GOALS
    }

    /// Validates that a string does not exceed the platform's title limit.
    /// @param title The campaign title to validate.
    /// @return `true` if length <= MAX_TITLE_LENGTH.
    pub fn validate_title(_env: Env, title: String) -> bool {
        title.len() <= MAX_TITLE_LENGTH
    }

    /// Validates that a description does not exceed the platform limit.
    /// @param description The campaign description to validate.
    /// @return `true` if length <= MAX_DESCRIPTION_LENGTH.
    pub fn validate_description(_env: Env, description: String) -> bool {
        description.len() <= MAX_DESCRIPTION_LENGTH
    }

    /// Validates that an aggregate metadata length is within bounds.
    /// @param total_len The combined length of all metadata strings.
    /// @return `true` if within safe limits to prevent state-rent spikes.
    pub fn validate_metadata_aggregate(_env: Env, total_len: u32) -> bool {
        const AGGREGATE_LIMIT: u32 = MAX_TITLE_LENGTH + MAX_DESCRIPTION_LENGTH + MAX_SOCIAL_LINKS_LENGTH;
        total_len <= AGGREGATE_LIMIT
    }
}
