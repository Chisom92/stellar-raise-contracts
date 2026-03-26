//! Proptest Generator Boundary Contract
//!
//! This contract provides the source of truth for all boundary conditions and validation
//! constants used by the crowdfunding platform's property-based tests. Exposing these
//! via a contract allows off-chain scripts and other contracts to dynamically query
//! current safe operating limits.
//!
//! ## Security
//!
//! - **Immutable Boundaries**: Constants are defined at compile-time to ensure test stability.
//! - **Public Transparency**: All limits are publicly readable.
//! - **Safety Guards**: Includes logic to clamp and validate inputs against platform-wide floors and caps.

#![no_std]

use soroban_sdk::{contract, contractimpl, Env, Symbol};

// ── Constants ────────────────────────────────────────────────────────────────

pub const DEADLINE_OFFSET_MIN: u64 = 1_000;
pub const DEADLINE_OFFSET_MAX: u64 = 1_000_000;
pub const GOAL_MIN: i128 = 1_000;
pub const GOAL_MAX: i128 = 100_000_000;
pub const MIN_CONTRIBUTION_FLOOR: i128 = 1;
pub const PROGRESS_BPS_CAP: u32 = 10_000;
pub const FEE_BPS_CAP: u32 = 10_000;
pub const PROPTEST_CASES_MIN: u32 = 32;
pub const PROPTEST_CASES_MAX: u32 = 256;
pub const GENERATOR_BATCH_MAX: u32 = 512;

#[contract]
pub struct ProptestGeneratorBoundary;

#[contractimpl]
impl ProptestGeneratorBoundary {
    /// Returns the minimum deadline offset in seconds.
    pub fn deadline_offset_min(_env: Env) -> u64 {
        DEADLINE_OFFSET_MIN
    }

    /// Returns the maximum deadline offset in seconds.
    pub fn deadline_offset_max(_env: Env) -> u64 {
        DEADLINE_OFFSET_MAX
    }

    /// Returns the minimum valid goal amount.
    pub fn goal_min(_env: Env) -> i128 {
        GOAL_MIN
    }

    /// Returns the maximum goal amount for test generations.
    pub fn goal_max(_env: Env) -> i128 {
        GOAL_MAX
    }

    /// Returns the absolute floor for contribution amounts.
    pub fn min_contribution_floor(_env: Env) -> i128 {
        MIN_CONTRIBUTION_FLOOR
    }

    /// Validates that an offset is within the [min, max] range.
    pub fn is_valid_deadline_offset(_env: Env, offset: u64) -> bool {
        (DEADLINE_OFFSET_MIN..=DEADLINE_OFFSET_MAX).contains(&offset)
    }

    /// Validates that a goal is within the [min, max] range.
    pub fn is_valid_goal(_env: Env, goal: i128) -> bool {
        (GOAL_MIN..=GOAL_MAX).contains(&goal)
    }

    /// Clamps a requested proptest case count into safe operating bounds.
    pub fn clamp_proptest_cases(_env: Env, requested: u32) -> u32 {
        requested.clamp(PROPTEST_CASES_MIN, PROPTEST_CASES_MAX)
    }

    /// Computes progress in basis points, capped at 10,000.
    pub fn compute_progress_bps(_env: Env, raised: i128, goal: i128) -> u32 {
        if goal <= 0 {
            return 0;
        }
        let raw = raised.saturating_mul(10_000) / goal;
        if raw <= 0 {
            0
        } else if raw >= PROGRESS_BPS_CAP as i128 {
            PROGRESS_BPS_CAP
        } else {
            raw as u32
        }
    }

    /// Returns a diagnostic tag for boundary log events.
    pub fn log_tag(_env: Env) -> Symbol {
        Symbol::new(&_env, "boundary")
    }
}
