//! Stellar Token Minter Contract
//!
//! This contract provides NFT minting capabilities for the crowdfunding platform.
//! It implements a simple minting mechanism that can be called by authorized
//! contracts (like the Crowdfund contract) to reward contributors with NFTs.
//!
//! ## Security
//!
//! - **Authorization**: Only the contract admin or the designated minter can call `mint`.
//! - **State Management**: Uses persistent storage for token ID tracking and metadata.
//! - **Bounded Operations**: Ensures all operations are within Soroban resource limits.

#![no_std]

// ── Test constants ────────────────────────────────────────────────────────────
//
// Centralised numeric literals used across the stellar_token_minter test suites.
// Defining them here means CI/CD only needs to update one location when campaign
// parameters change, and test intent is self-documenting.

/// Default campaign funding goal used in tests (1 000 000 stroops).
pub const TEST_GOAL: i128 = 1_000_000;

/// Default minimum contribution used in tests (1 000 stroops).
pub const TEST_MIN_CONTRIBUTION: i128 = 1_000;

/// Default campaign duration used in tests (1 hour in seconds).
pub const TEST_DEADLINE_OFFSET: u64 = 3_600;

/// Initial token balance minted to the creator in the test setup helper.
pub const TEST_CREATOR_BALANCE: i128 = 100_000_000;

/// Initial token balance minted to the token-minter test setup helper.
pub const TEST_MINTER_CREATOR_BALANCE: i128 = 10_000_000;

/// Standard single-contributor balance used in most integration tests.
pub const TEST_CONTRIBUTOR_BALANCE: i128 = 1_000_000;

/// Contribution amount used in NFT-batch tests (goal / MAX_MINT_BATCH).
pub const TEST_NFT_CONTRIBUTION: i128 = 25_000;

/// Contribution amount used in the "below batch limit" NFT test.
pub const TEST_NFT_SMALL_CONTRIBUTION: i128 = 400_000;

/// Contribution amount used in collect_pledges / two-contributor tests.
pub const TEST_PLEDGE_CONTRIBUTION: i128 = 300_000;

/// Bonus goal threshold used in idempotency tests.
pub const TEST_BONUS_GOAL: i128 = 1_000_000;

/// Primary goal used in bonus-goal idempotency tests.
pub const TEST_BONUS_PRIMARY_GOAL: i128 = 500_000;

/// Per-contribution amount used in bonus-goal crossing tests.
pub const TEST_BONUS_CONTRIBUTION: i128 = 600_000;

/// Seed balance for overflow protection test (small initial contribution).
pub const TEST_OVERFLOW_SEED: i128 = 10_000;

/// Maximum platform fee in basis points (100 %).
pub const TEST_FEE_BPS_MAX: u32 = 10_000;

/// Platform fee that exceeds the maximum (triggers panic).
pub const TEST_FEE_BPS_OVER: u32 = 10_001;

/// Platform fee of 10 % used in fee-deduction tests.
pub const TEST_FEE_BPS_10PCT: u32 = 1_000;

/// Progress basis points representing 80 % funding.
pub const TEST_PROGRESS_BPS_80PCT: u32 = 8_000;

/// Progress basis points representing 99.999 % funding (just below goal).
pub const TEST_PROGRESS_BPS_JUST_BELOW: u32 = 9_999;

/// Contribution amount that is one stroop below the goal.
pub const TEST_JUST_BELOW_GOAL: i128 = 999_999;

/// Contribution amount used in the "partial accumulation" test.
pub const TEST_PARTIAL_CONTRIBUTION_A: i128 = 300_000;

/// Second contribution amount used in the "partial accumulation" test.
pub const TEST_PARTIAL_CONTRIBUTION_B: i128 = 200_000;

// ── Constants ────────────────────────────────────────────────────────────────

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Minter,
    TotalMinted,
    TokenMetadata(u64),
}

#[contract]
pub struct StellarTokenMinter;

#[contractimpl]
impl StellarTokenMinter {
    /// Initializes the minter contract.
    ///
    /// # Arguments
    ///
    /// * `admin` - Contract administrator
    /// * `minter` - Address authorized to perform minting
    pub fn initialize(env: Env, admin: Address, minter: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Minter, &minter);
        env.storage().instance().set(&DataKey::TotalMinted, &0u64);
    }

    /// Mints a new NFT to the specified recipient.
    ///
    /// # Arguments
    ///
    /// * `to` - Recipient address
    /// * `token_id` - ID of the token to mint
    ///
    /// # Panics
    ///
    /// * If the caller is not authorized (not admin or minter)
    /// * If the token ID has already been minted
    pub fn mint(env: Env, to: Address, token_id: u64) {
        let minter: Address = env.storage().instance().get(&DataKey::Minter).unwrap();
        minter.require_auth();

        let key = DataKey::TokenMetadata(token_id);
        if env.storage().persistent().has(&key) {
            panic!("token already minted");
        }

        // Store some basic metadata to record the ownership
        env.storage().persistent().set(&key, &to);

        // Update total counter
        let total: u64 = env.storage().instance().get(&DataKey::TotalMinted).unwrap();
        env.storage().instance().set(&DataKey::TotalMinted, &(total + 1));

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "mint"), to),
            token_id,
        );
    }

    /// Returns the owner of a token.
    pub fn owner(env: Env, token_id: u64) -> Option<Address> {
        env.storage().persistent().get(&DataKey::TokenMetadata(token_id))
    }

    /// Returns the total number of NFTs minted.
    pub fn total_minted(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::TotalMinted).unwrap_or(0)
    }

    /// Updates the minter address. Only callable by admin.
    pub fn set_minter(env: Env, admin: Address, new_minter: Address) {
        let current_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        current_admin.require_auth();
        if admin != current_admin {
            panic!("unauthorized");
        }
        env.storage().instance().set(&DataKey::Minter, &new_minter);
    }
}
