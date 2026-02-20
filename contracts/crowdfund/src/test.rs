#![cfg(test)]

use soroban_sdk::{testutils::{Address as _, Ledger}, token, Address, Env};

use crate::{CrowdfundContract, CrowdfundContractClient};

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Set up a fresh environment with a deployed crowdfund contract and a token.
fn setup_env() -> (Env, CrowdfundContractClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    // Deploy the crowdfund contract.
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    // Create a token for contributions.
    let token_admin = Address::generate(&env);
    let token_contract_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_contract_id.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    // Campaign creator.
    let creator = Address::generate(&env);

    // Mint tokens to the creator so the contract has something to work with.
    token_admin_client.mint(&creator, &10_000_000);

    (env, client, creator, token_address, token_admin.clone())
}

/// Helper to mint tokens to an arbitrary contributor.
fn mint_to(env: &Env, token_address: &Address, admin: &Address, to: &Address, amount: i128) {
    let admin_client = token::StellarAssetClient::new(env, token_address);
    admin_client.mint(to, &amount);
    let _ = admin;
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn test_initialize() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600; // 1 hour from now
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;

    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    assert_eq!(client.goal(), goal);
    assert_eq!(client.deadline(), deadline);
    assert_eq!(client.min_contribution(), min_contribution);
    assert_eq!(client.total_raised(), 0);
}

#[test]
fn test_double_initialize_panics() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;

    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution); // should panic
}

#[test]
fn test_contribute() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);

    client.contribute(&contributor, &500_000);

    assert_eq!(client.total_raised(), 500_000);
    assert_eq!(client.contribution(&contributor), 500_000);
}

#[test]
fn test_multiple_contributions() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &alice, 600_000);
    mint_to(&env, &token_address, &admin, &bob, 400_000);

    client.contribute(&alice, &600_000);
    client.contribute(&bob, &400_000);

    assert_eq!(client.total_raised(), 1_000_000);
    assert_eq!(client.contribution(&alice), 600_000);
    assert_eq!(client.contribution(&bob), 400_000);
}

#[test]
fn test_contribute_after_deadline_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 100;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    // Fast-forward past the deadline.
    env.ledger().set_timestamp(deadline + 1);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);

    let result = client.try_contribute(&contributor, &500_000);
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), crate::ContractError::CampaignEnded);
}

#[test]
fn test_withdraw_after_goal_met() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    assert_eq!(client.total_raised(), goal);

    // Move past deadline.
    env.ledger().set_timestamp(deadline + 1);

    client.withdraw();

    // After withdrawal, total_raised resets to 0.
    assert_eq!(client.total_raised(), 0);

    // Creator should have received the funds.
    let token_client = token::Client::new(&env, &token_address);
    assert_eq!(token_client.balance(&creator), 10_000_000 + 1_000_000);
}

#[test]
fn test_withdraw_before_deadline_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    let result = client.try_withdraw();
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), crate::ContractError::CampaignStillActive);
}

#[test]
fn test_withdraw_goal_not_reached_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);
    client.contribute(&contributor, &500_000);

    // Move past deadline, but goal not met.
    env.ledger().set_timestamp(deadline + 1);

    let result = client.try_withdraw();
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), crate::ContractError::GoalNotReached);
}

#[test]
fn test_refund_when_goal_not_met() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &alice, 300_000);
    mint_to(&env, &token_address, &admin, &bob, 200_000);

    client.contribute(&alice, &300_000);
    client.contribute(&bob, &200_000);

    // Move past deadline — goal not met.
    env.ledger().set_timestamp(deadline + 1);

    client.refund();

    // Both contributors should get their tokens back.
    let token_client = token::Client::new(&env, &token_address);
    assert_eq!(token_client.balance(&alice), 300_000);
    assert_eq!(token_client.balance(&bob), 200_000);
    assert_eq!(client.total_raised(), 0);
}

#[test]
fn test_refund_when_goal_reached_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline + 1);

    let result = client.try_refund();
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), crate::ContractError::GoalReached);
}

// ── Bug Condition Exploration Tests ────────────────────────────────────────
//
// **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5**
//
// These tests demonstrate the bug on UNFIXED code. They are EXPECTED TO FAIL
// because the current implementation accepts invalid inputs that should be rejected.
//
// CRITICAL: These tests encode the EXPECTED behavior (validation with errors).
// When run on unfixed code, they will fail because the code doesn't validate.
// After the fix is implemented, these tests should pass.

#[test]
fn test_bug_condition_zero_goal_rejected() {
    // **Property 1: Fault Condition** - Input Validation Rejection
    // **Validates: Requirements 2.1**
    
    let (env, client, creator, token_address, _admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    
    // Try to initialize with zero goal - should be rejected
    let result = client.try_initialize(&creator, &token_address, &0i128, &deadline);
    
    // Expected behavior: should return InvalidGoal error
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), crate::ContractError::InvalidGoal);
}

#[test]
fn test_bug_condition_negative_goal_rejected() {
    // **Property 1: Fault Condition** - Input Validation Rejection
    // **Validates: Requirements 2.2**
    
    let (env, client, creator, token_address, _admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    
    // Try to initialize with negative goal - should be rejected
    let result = client.try_initialize(&creator, &token_address, &-100i128, &deadline);
    
    // Expected behavior: should return InvalidGoal error
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), crate::ContractError::InvalidGoal);
}

#[test]
fn test_bug_condition_past_deadline_rejected() {
    // **Property 1: Fault Condition** - Input Validation Rejection
    // **Validates: Requirements 2.3**
    
    let (env, client, creator, token_address, _admin) = setup_env();
    let current_time = env.ledger().timestamp();
    // Set a past deadline (if current_time is 0, use 0 as past)
    let past_deadline = if current_time > 100 { current_time - 100 } else { 0 };
    
    // Try to initialize with past deadline - should be rejected
    let result = client.try_initialize(&creator, &token_address, &1_000_000i128, &past_deadline);
    
    // Expected behavior: should return InvalidDeadline error
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), crate::ContractError::InvalidDeadline);
}

#[test]
fn test_bug_condition_current_timestamp_deadline_rejected() {
    // **Property 1: Fault Condition** - Input Validation Rejection
    // **Validates: Requirements 2.3**
    
    let (env, client, creator, token_address, _admin) = setup_env();
    let current_time = env.ledger().timestamp();
    
    // Try to initialize with deadline equal to current time - should be rejected
    let result = client.try_initialize(&creator, &token_address, &1_000_000i128, &current_time);
    
    // Expected behavior: should return InvalidDeadline error
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), crate::ContractError::InvalidDeadline);
}

#[test]
fn test_bug_condition_zero_amount_contribution_rejected() {
    // **Property 1: Fault Condition** - Input Validation Rejection
    // **Validates: Requirements 2.4**
    
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    client.initialize(&creator, &token_address, &1_000_000i128, &deadline);
    
    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 100_000);
    
    // Try to contribute zero amount - should be rejected
    let result = client.try_contribute(&contributor, &0i128);
    
    // Expected behavior: should return InvalidAmount error
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), crate::ContractError::InvalidAmount);
}

#[test]
fn test_bug_condition_negative_amount_contribution_rejected() {
    // **Property 1: Fault Condition** - Input Validation Rejection
    // **Validates: Requirements 2.5**
    
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    client.initialize(&creator, &token_address, &1_000_000i128, &deadline);
    
    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 100_000);
    
    // Try to contribute negative amount - should be rejected
    let result = client.try_contribute(&contributor, &-50i128);
    
    // Expected behavior: should return InvalidAmount error
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), crate::ContractError::InvalidAmount);
}

// ── Preservation Property Tests ────────────────────────────────────────────
//
// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5**
//
// **Property 2: Preservation** - Valid Input Acceptance
//
// These tests verify that all successful execution paths work correctly
// on the UNFIXED code. These behaviors MUST be preserved after the fix.
//
// IMPORTANT: These tests are EXPECTED TO PASS on unfixed code.
// When they pass, they confirm the baseline behavior to preserve.

use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_preservation_valid_initialization(
        goal in 1i128..10_000_000i128,
        deadline_offset in 1u64..10_000u64,
    ) {
        // **Property 2: Preservation** - Valid Input Acceptance
        // **Validates: Requirement 3.1**
        //
        // This test verifies that valid initialization (goal > 0, deadline > now)
        // continues to work correctly after the fix.
        
        let (env, client, creator, token_address, _admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;
        
        // Valid initialization should succeed
        client.initialize(&creator, &token_address, &goal, &deadline);
        
        // Verify state is stored correctly
        prop_assert_eq!(client.goal(), goal);
        prop_assert_eq!(client.deadline(), deadline);
        prop_assert_eq!(client.total_raised(), 0);
    }
    
    #[test]
    fn prop_preservation_valid_contribution(
        goal in 1_000_000i128..10_000_000i128,
        deadline_offset in 100u64..10_000u64,
        contribution_amount in 1i128..1_000_000i128,
    ) {
        // **Property 2: Preservation** - Valid Input Acceptance
        // **Validates: Requirement 3.2**
        //
        // This test verifies that valid contributions (amount > 0, before deadline)
        // continue to work correctly after the fix.
        
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;
        
        client.initialize(&creator, &token_address, &goal, &deadline);
        
        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, contribution_amount);
        
        // Valid contribution should succeed
        client.contribute(&contributor, &contribution_amount);
        
        // Verify state is updated correctly
        prop_assert_eq!(client.total_raised(), contribution_amount);
        prop_assert_eq!(client.contribution(&contributor), contribution_amount);
    }
    
    #[test]
    fn prop_preservation_existing_error_paths(
        goal in 1_000_000i128..10_000_000i128,
        deadline_offset in 100u64..1000u64,
    ) {
        // **Property 2: Preservation** - Valid Input Acceptance
        // **Validates: Requirement 3.3**
        //
        // This test verifies that existing error paths (like CampaignEnded)
        // continue to work correctly after the fix.
        
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;
        
        client.initialize(&creator, &token_address, &goal, &deadline);
        
        // Move past deadline
        env.ledger().set_timestamp(deadline + 1);
        
        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, 100_000);
        
        // Contribution after deadline should still fail with CampaignEnded
        let result = client.try_contribute(&contributor, &100_000);
        prop_assert!(result.is_err());
    }
}

#[test]
#[should_panic(expected = "campaign is not active")]
fn test_double_withdraw_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline + 1);

    client.withdraw();
    client.withdraw(); // should panic — status is Successful
}

#[test]
#[should_panic(expected = "campaign is not active")]
fn test_double_refund_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let alice = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &alice, 500_000);
    client.contribute(&alice, &500_000);

    env.ledger().set_timestamp(deadline + 1);

    client.refund();
    client.refund(); // should panic — status is Refunded
}

#[test]
fn test_cancel_with_no_contributions() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    client.cancel();

    assert_eq!(client.total_raised(), 0);
}

#[test]
fn test_cancel_with_contributions() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &alice, 300_000);
    mint_to(&env, &token_address, &admin, &bob, 200_000);

    client.contribute(&alice, &300_000);
    client.contribute(&bob, &200_000);

    client.cancel();

    let token_client = token::Client::new(&env, &token_address);
    assert_eq!(token_client.balance(&alice), 300_000);
    assert_eq!(token_client.balance(&bob), 200_000);
    assert_eq!(client.total_raised(), 0);
}

#[test]
#[should_panic]
fn test_cancel_by_non_creator_panics() {
    let env = Env::default();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_contract_id.address();

    let creator = Address::generate(&env);
    let non_creator = Address::generate(&env);

    env.mock_all_auths();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    env.mock_all_auths_allowing_non_root_auth();
    env.set_auths(&[]);
    
    client.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &non_creator,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &contract_id,
            fn_name: "cancel",
            args: soroban_sdk::vec![&env],
            sub_invokes: &[],
        },
    }]);

    client.cancel();
}

// ── Minimum Contribution Tests ─────────────────────────────────────────────

#[test]
#[should_panic(expected = "amount below minimum")]
fn test_contribute_below_minimum_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 10_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 5_000);

    client.contribute(&contributor, &5_000); // should panic
}

#[test]
fn test_contribute_exact_minimum() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 10_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 10_000);

    client.contribute(&contributor, &10_000);

    assert_eq!(client.total_raised(), 10_000);
    assert_eq!(client.contribution(&contributor), 10_000);
}

#[test]
fn test_contribute_above_minimum() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 10_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 50_000);

    client.contribute(&contributor, &50_000);

    assert_eq!(client.total_raised(), 50_000);
    assert_eq!(client.contribution(&contributor), 50_000);
}

// ── Campaign Stats Tests ───────────────────────────────────────────────────

#[test]
fn test_stats_no_contributions() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let stats = client.get_stats();

    assert_eq!(stats.total_raised, 0);
    assert_eq!(stats.goal, 1_000_000);
    assert_eq!(stats.progress_bps, 0);
    assert_eq!(stats.contributor_count, 0);
    assert_eq!(stats.average_contribution, 0);
    assert_eq!(stats.largest_contribution, 0);
}

#[test]
fn test_stats_single_contributor() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);
    client.contribute(&contributor, &500_000);

    let stats = client.get_stats();

    assert_eq!(stats.total_raised, 500_000);
    assert_eq!(stats.goal, 1_000_000);
    assert_eq!(stats.progress_bps, 5_000); // 50%
    assert_eq!(stats.contributor_count, 1);
    assert_eq!(stats.average_contribution, 500_000);
    assert_eq!(stats.largest_contribution, 500_000);
}

#[test]
fn test_stats_multiple_contributors() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let charlie = Address::generate(&env);
    
    mint_to(&env, &token_address, &admin, &alice, 600_000);
    mint_to(&env, &token_address, &admin, &bob, 300_000);
    mint_to(&env, &token_address, &admin, &charlie, 100_000);

    client.contribute(&alice, &600_000);
    client.contribute(&bob, &300_000);
    client.contribute(&charlie, &100_000);

    let stats = client.get_stats();

    assert_eq!(stats.total_raised, 1_000_000);
    assert_eq!(stats.goal, 1_000_000);
    assert_eq!(stats.progress_bps, 10_000); // 100%
    assert_eq!(stats.contributor_count, 3);
    assert_eq!(stats.average_contribution, 333_333); // 1_000_000 / 3
    assert_eq!(stats.largest_contribution, 600_000);
}

#[test]
fn test_stats_progress_capped_at_10000() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_500_000);
    client.contribute(&contributor, &1_500_000);

    let stats = client.get_stats();

    assert_eq!(stats.total_raised, 1_500_000);
    assert_eq!(stats.goal, 1_000_000);
    assert_eq!(stats.progress_bps, 10_000); // Capped at 100%
    assert_eq!(stats.contributor_count, 1);
    assert_eq!(stats.average_contribution, 1_500_000);
    assert_eq!(stats.largest_contribution, 1_500_000);
}
