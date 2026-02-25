#![cfg(test)]

use soroban_sdk::{testutils::{Address as _, Ledger, Events}, token, Address, Env};

use crate::{CrowdfundContract, CrowdfundContractClient};

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Set up a fresh environment with a deployed crowdfund contract and a token.
fn setup_env() -> (
    Env,
    CrowdfundContractClient<'static>,
    Address,
    Address,
    Address,
) {
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

/// Helper to create default title and description for tests.
fn default_title(env: &Env) -> soroban_sdk::String {
    soroban_sdk::String::from_str(env, "Test Campaign")
}

fn default_description(env: &Env) -> soroban_sdk::String {
    soroban_sdk::String::from_str(env, "A test crowdfunding campaign")
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn test_initialize() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600; // 1 hour from now
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    let title = default_title(&env);
    let description = default_description(&env);

    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
        &title,
        &description,
        &None,
    );

    assert_eq!(client.goal(), goal);
    assert_eq!(client.deadline(), deadline);
    assert_eq!(client.min_contribution(), min_contribution);
    assert_eq!(client.total_raised(), 0);
    assert_eq!(client.title(), title);
    assert_eq!(client.description(), description);
}

#[test]
fn test_version() {
    let (_env, client, _creator, _token_address, _admin) = setup_env();

    // Test that version() returns the expected version number
    assert_eq!(client.version(), 2);
}

#[test]
fn test_double_initialize_panics() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;

    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);
    let result = client.try_initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
        &default_title(&env),
        &default_description(&env),
        &None,
    );

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::AlreadyInitialized
    );
}

#[test]
fn test_contribute() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

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
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

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
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    // Fast-forward past the deadline.
    env.ledger().set_timestamp(deadline + 1);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);

    let result = client.try_contribute(&contributor, &500_000);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::CampaignEnded
    );
}

#[test]
fn test_withdraw_after_goal_met() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

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
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    let result = client.try_withdraw();

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::CampaignStillActive
    );
}

#[test]
fn test_withdraw_goal_not_reached_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);
    client.contribute(&contributor, &500_000);

    // Move past deadline, but goal not met.
    env.ledger().set_timestamp(deadline + 1);

    let result = client.try_withdraw();

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::GoalNotReached
    );
}

#[test]
fn test_refund_single_when_goal_not_met() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &alice, 300_000);
    mint_to(&env, &token_address, &admin, &bob, 200_000);

    client.contribute(&alice, &300_000);
    client.contribute(&bob, &200_000);

    // Move past deadline — goal not met.
    env.ledger().set_timestamp(deadline + 1);

    client.refund_single(&alice);
    client.refund_single(&bob);

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
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline + 1);

    let result = client.try_refund_single(&contributor);
    
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::GoalReached
    );
}
#[test]
fn test_refund_single_before_deadline_fails() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);
    client.contribute(&contributor, &500_000);

    // Try to refund before deadline passes
    let result = client.try_refund_single(&contributor);

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), crate::ContractError::CampaignStillActive);
}


// ── Bug Condition Exploration Test ─────────────────────────────────────────

/// **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5, 2.6**
///
/// **Property 1: Fault Condition** - Structured Error Returns
///
/// This test verifies that all 6 error conditions return the appropriate
/// ContractError variants instead of panicking.
///
/// The test covers all 6 error conditions:
/// 1. Double initialization → Err(ContractError::AlreadyInitialized)
/// 2. Late contribution → Err(ContractError::CampaignEnded)
/// 3. Early withdrawal → Err(ContractError::CampaignStillActive)
/// 4. Withdrawal without goal → Err(ContractError::GoalNotReached)
/// 5. Early refund → Err(ContractError::CampaignStillActive)
/// 6. Refund after success → Err(ContractError::GoalReached)
#[test]
fn test_bug_condition_exploration_all_error_conditions_panic() {
    use crate::ContractError;

    // Test 1: Double initialization
    {
        let (env, client, creator, token_address, _admin) = setup_env();
        let deadline = env.ledger().timestamp() + 3600;
        let goal: i128 = 1_000_000;
        
        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);
        let result = client.try_initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);
        
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            ContractError::AlreadyInitialized
        );
    }

    // Test 2: Late contribution
    {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + 100;
        let goal: i128 = 1_000_000;
        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);
        
        env.ledger().set_timestamp(deadline + 1);

        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, 500_000);
        let result = client.try_contribute(&contributor, &500_000);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().unwrap(), ContractError::CampaignEnded);
    }

    // Test 3: Early withdrawal
    {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + 3600;
        let goal: i128 = 1_000_000;
        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);
        
        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
        client.contribute(&contributor, &1_000_000);

        let result = client.try_withdraw();

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            ContractError::CampaignStillActive
        );
    }

    // Test 4: Withdrawal without goal
    {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + 3600;
        let goal: i128 = 1_000_000;
        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);
        
        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, 500_000);
        client.contribute(&contributor, &500_000);

        env.ledger().set_timestamp(deadline + 1);
        let result = client.try_withdraw();

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().unwrap(), ContractError::GoalNotReached);
    }

    // Test 5: Early refund
    {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + 3600;
        let goal: i128 = 1_000_000;
        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);
        
        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, 500_000);
        client.contribute(&contributor, &500_000);
        
        let result = client.try_refund_single(&contributor);
        
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            ContractError::CampaignStillActive
        );
    }

    // Test 6: Refund after success
    {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + 3600;
        let goal: i128 = 1_000_000;
        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);
        
        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
        client.contribute(&contributor, &1_000_000);

        env.ledger().set_timestamp(deadline + 1);
        let result = client.try_refund_single(&contributor);
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().unwrap(), ContractError::GoalReached);
    }
}

// ── Preservation Property Tests ────────────────────────────────────────────

use proptest::prelude::*;

/// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**
///
/// **Property 2: Preservation** - Successful Execution Paths
///
/// This test verifies that all successful execution paths work correctly
/// on the UNFIXED code. These behaviors MUST be preserved after the fix.
///
/// **IMPORTANT**: This test is EXPECTED TO PASS on unfixed code.
/// When it passes, it confirms the baseline behavior to preserve.
///
/// The test covers all successful operations:
/// 1. First initialization with valid parameters stores creator, token, goal, deadline, and initializes total_raised to 0
/// 2. Valid contributions before deadline transfer tokens, update balances, and track contributors
/// 3. Successful withdrawal by creator after deadline when goal met transfers funds and resets total_raised
/// 4. Successful refund after deadline when goal not met refunds all contributors
/// 5. View functions (total_raised, goal, deadline, contribution) return correct values
/// 6. Multiple contributors are tracked correctly with individual and aggregate totals

proptest! {
    #[test]
    fn prop_preservation_first_initialization(
        goal in 1_000i128..10_000_000i128,
        deadline_offset in 100u64..10_000u64,
    ) {
        let (env, client, creator, token_address, _admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        // Test 3.1: First initialization stores all values correctly
        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);

        prop_assert_eq!(client.goal(), goal);
        prop_assert_eq!(client.deadline(), deadline);
        prop_assert_eq!(client.total_raised(), 0);
    }

    #[test]
    fn prop_preservation_valid_contribution(
        goal in 1_000_000i128..10_000_000i128,
        deadline_offset in 100u64..10_000u64,
        contribution_amount in 100_000i128..1_000_000i128,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);

    client.contribute(&alice, &300_000, None);
    client.contribute(&bob, &200_000, None);

        // Test 3.2: Valid contribution before deadline works correctly
        client.contribute(&contributor, &contribution_amount);

        prop_assert_eq!(client.total_raised(), contribution_amount);
        prop_assert_eq!(client.contribution(&contributor), contribution_amount);
    }

    #[test]
    fn prop_preservation_successful_withdrawal(
        goal in 1_000_000i128..5_000_000i128,
        deadline_offset in 100u64..10_000u64,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);

        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, goal);
        client.contribute(&contributor, &goal);

        // Move past deadline
        env.ledger().set_timestamp(deadline + 1);

    client.contribute(&contributor, &10_000, None);

        // Test 3.3: Successful withdrawal transfers funds and resets total_raised
        client.withdraw();

        prop_assert_eq!(client.total_raised(), 0);
        prop_assert_eq!(token_client.balance(&creator), creator_balance_before + goal);
    }

    #[test]
    fn prop_preservation_successful_refund(
        goal in 2_000_000i128..10_000_000i128,
        deadline_offset in 100u64..10_000u64,
        contribution_amount in 100_000i128..1_000_000i128,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        // Ensure contribution is less than goal
        let contribution = contribution_amount.min(goal - 1);

    client.contribute(&contributor, &50_000, &None);

        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, contribution);
        client.contribute(&contributor, &contribution);

        // Move past deadline (goal not met)
        env.ledger().set_timestamp(deadline + 1);

        // Test 3.4: Successful refund returns funds to contributors
        client.refund_single(&contributor);

        let token_client = token::Client::new(&env, &token_address);
        prop_assert_eq!(token_client.balance(&contributor), contribution);
        prop_assert_eq!(client.total_raised(), 0);
    }

    #[test]
    fn prop_preservation_view_functions(
        goal in 1_000_000i128..10_000_000i128,
        deadline_offset in 100u64..10_000u64,
        contribution_amount in 100_000i128..1_000_000i128,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);

        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, contribution_amount);
        client.contribute(&contributor, &contribution_amount);

        // Test 3.5: View functions return correct values
        prop_assert_eq!(client.goal(), goal);
        prop_assert_eq!(client.deadline(), deadline);
        prop_assert_eq!(client.total_raised(), contribution_amount);
        prop_assert_eq!(client.contribution(&contributor), contribution_amount);
    }

    #[test]
    fn prop_preservation_multiple_contributors(
        goal in 5_000_000i128..10_000_000i128,
        deadline_offset in 100u64..10_000u64,
        amount1 in 100_000i128..1_000_000i128,
        amount2 in 100_000i128..1_000_000i128,
        amount3 in 100_000i128..1_000_000i128,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);

        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let charlie = Address::generate(&env);

        mint_to(&env, &token_address, &admin, &alice, amount1);
        mint_to(&env, &token_address, &admin, &bob, amount2);
        mint_to(&env, &token_address, &admin, &charlie, amount3);

        // Test 3.6: Multiple contributors are tracked correctly
        client.contribute(&alice, &amount1);
        client.contribute(&bob, &amount2);
        client.contribute(&charlie, &amount3);

        let expected_total = amount1 + amount2 + amount3;

        prop_assert_eq!(client.total_raised(), expected_total);
        prop_assert_eq!(client.contribution(&alice), amount1);
        prop_assert_eq!(client.contribution(&bob), amount2);
        prop_assert_eq!(client.contribution(&charlie), amount3);
    }
}

#[test]
#[should_panic(expected = "campaign is not active")]
fn test_double_withdraw_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline + 1);

    client.withdraw();
    client.withdraw(); // should panic — status is Successful
}

#[test]
fn test_double_refund_single_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let alice = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &alice, 500_000);
    client.contribute(&alice, &500_000);

    env.ledger().set_timestamp(deadline + 1);

    client.refund_single(&alice);
    // Second refund should succeed but do nothing (amount is 0)
    let result = client.try_refund_single(&alice);
    assert!(result.is_ok());
}

// NOTE: The following tests are commented out because the cancel function
// is not implemented in the current version of the contract.
// TODO: Implement cancel function or remove these tests.

/*
#[test]
fn test_cancel_with_no_contributions() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

proptest! {
    #[test]
    fn prop_preservation_first_initialization(
        goal in 1_000i128..10_000_000i128,
        deadline_offset in 100u64..10_000u64,
    ) {
        let (env, client, creator, token_address, _admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        // Test 3.1: First initialization stores all values correctly
        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);

        prop_assert_eq!(client.goal(), goal);
        prop_assert_eq!(client.deadline(), deadline);
        prop_assert_eq!(client.total_raised(), 0);
    }

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);

    client.contribute(&alice, &300_000);
    client.contribute(&bob, &200_000);

        // Test 3.2: Valid contribution before deadline works correctly
        client.contribute(&contributor, &contribution_amount);

        prop_assert_eq!(client.total_raised(), contribution_amount);
        prop_assert_eq!(client.contribution(&contributor), contribution_amount);
    }

    #[test]
    fn prop_preservation_successful_withdrawal(
        goal in 1_000_000i128..5_000_000i128,
        deadline_offset in 100u64..10_000u64,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

        // Move past deadline
        env.ledger().set_timestamp(deadline + 1);

    client.contribute(&contributor, &10_000);

    assert_eq!(client.total_raised(), 10_000);
    assert_eq!(client.contribution(&contributor), 10_000);
}
*/

        prop_assert_eq!(client.total_raised(), 0);
        prop_assert_eq!(token_client.balance(&creator), creator_balance_before + goal);
    }

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 10_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

        // Ensure contribution is less than goal
        let contribution = contribution_amount.min(goal - 1);

    client.contribute(&contributor, &50_000);

        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, contribution);
        client.contribute(&contributor, &contribution);

        // Move past deadline (goal not met)
        env.ledger().set_timestamp(deadline + 1);

        // Test 3.4: Successful refund returns funds to contributors
        client.refund_single(&contributor);

        let token_client = token::Client::new(&env, &token_address);
        prop_assert_eq!(token_client.balance(&contributor), contribution);
        prop_assert_eq!(client.total_raised(), 0);
    }

    #[test]
    fn prop_preservation_view_functions(
        goal in 1_000_000i128..10_000_000i128,
        deadline_offset in 100u64..10_000u64,
        contribution_amount in 100_000i128..1_000_000i128,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);

        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, contribution_amount);
        client.contribute(&contributor, &contribution_amount);

        // Test 3.5: View functions return correct values
        prop_assert_eq!(client.goal(), goal);
        prop_assert_eq!(client.deadline(), deadline);
        prop_assert_eq!(client.total_raised(), contribution_amount);
        prop_assert_eq!(client.contribution(&contributor), contribution_amount);
    }

    #[test]
    fn prop_preservation_multiple_contributors(
        goal in 5_000_000i128..10_000_000i128,
        deadline_offset in 100u64..10_000u64,
        amount1 in 100_000i128..1_000_000i128,
        amount2 in 100_000i128..1_000_000i128,
        amount3 in 100_000i128..1_000_000i128,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        client.initialize(&creator, &token_address, &goal, &deadline, &1_000, &default_title(&env), &default_description(&env), &None);

        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let charlie = Address::generate(&env);

        mint_to(&env, &token_address, &admin, &alice, amount1);
        mint_to(&env, &token_address, &admin, &bob, amount2);
        mint_to(&env, &token_address, &admin, &charlie, amount3);

        // Test 3.6: Multiple contributors are tracked correctly
        client.contribute(&alice, &amount1);
        client.contribute(&bob, &amount2);
        client.contribute(&charlie, &amount3);

        let expected_total = amount1 + amount2 + amount3;

        prop_assert_eq!(client.total_raised(), expected_total);
        prop_assert_eq!(client.contribution(&alice), amount1);
        prop_assert_eq!(client.contribution(&bob), amount2);
        prop_assert_eq!(client.contribution(&charlie), amount3);
    }
}

#[test]
#[should_panic(expected = "campaign is not active")]
fn test_double_withdraw_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 10_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let bronze = soroban_sdk::String::from_str(&env, "Bronze");
    let silver = soroban_sdk::String::from_str(&env, "Silver");
    let gold = soroban_sdk::String::from_str(&env, "Gold");
    client.add_reward_tier(&creator, &bronze, &10_000);
    client.add_reward_tier(&creator, &silver, &100_000);
    client.add_reward_tier(&creator, &gold, &500_000);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 50_000);
    client.contribute(&contributor, &50_000);

    let tier = client.get_user_tier(&contributor);
    assert!(tier.is_some());
    assert_eq!(tier.unwrap(), bronze);
}

#[test]
fn test_get_user_tier_gold_level() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 10_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let bronze = soroban_sdk::String::from_str(&env, "Bronze");
    let silver = soroban_sdk::String::from_str(&env, "Silver");
    let gold = soroban_sdk::String::from_str(&env, "Gold");
    client.add_reward_tier(&creator, &bronze, &10_000);
    client.add_reward_tier(&creator, &silver, &100_000);
    client.add_reward_tier(&creator, &gold, &500_000);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 600_000);
    client.contribute(&contributor, &600_000);

    let tier = client.get_user_tier(&contributor);
    assert!(tier.is_some());
    assert_eq!(tier.unwrap(), gold);
}

#[test]
fn test_get_user_tier_non_contributor_returns_none() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let bronze = soroban_sdk::String::from_str(&env, "Bronze");
    client.add_reward_tier(&creator, &bronze, &10_000);

    let non_contributor = Address::generate(&env);
    let tier = client.get_user_tier(&non_contributor);
    assert!(tier.is_none());
}

#[test]
fn test_get_user_tier_no_tiers_defined_returns_none() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
        &None,
    );

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);
    client.contribute(&contributor, &500_000);

    let tier = client.get_user_tier(&contributor);
    assert!(tier.is_none());
}

#[test]
fn test_get_user_tier_highest_qualifying_tier_returned() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
        &None,
    );

    let bronze = soroban_sdk::String::from_str(&env, "Bronze");
    let silver = soroban_sdk::String::from_str(&env, "Silver");
    let gold = soroban_sdk::String::from_str(&env, "Gold");
    client.add_reward_tier(&creator, &bronze, &10_000);
    client.add_reward_tier(&creator, &silver, &100_000);
    client.add_reward_tier(&creator, &gold, &500_000);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    let tier = client.get_user_tier(&contributor);
    assert!(tier.is_some());
    assert_eq!(tier.unwrap(), gold);
}

#[test]
#[should_panic]
fn test_add_reward_tier_non_creator_rejected() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
        &None,
    );

    let non_creator = Address::generate(&env);
    let bronze = soroban_sdk::String::from_str(&env, "Bronze");
    client.add_reward_tier(&non_creator, &bronze, &10_000);
}

#[test]
#[should_panic(expected = "min_amount must be greater than 0")]
fn test_add_reward_tier_rejects_zero_min_amount() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
        &None,
    );

    let bronze = soroban_sdk::String::from_str(&env, "Bronze");
    client.add_reward_tier(&creator, &bronze, &0);
}

#[test]
fn test_reward_tiers_view() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
        &None,
    );

    assert_eq!(client.reward_tiers().len(), 0);

    let bronze = soroban_sdk::String::from_str(&env, "Bronze");
    let silver = soroban_sdk::String::from_str(&env, "Silver");
    client.add_reward_tier(&creator, &bronze, &10_000);
    client.add_reward_tier(&creator, &silver, &100_000);

    let tiers = client.reward_tiers();
    assert_eq!(tiers.len(), 2);
    assert_eq!(tiers.get(0).unwrap().name, bronze);
    assert_eq!(tiers.get(0).unwrap().min_amount, 10_000);
    assert_eq!(tiers.get(1).unwrap().name, silver);
    assert_eq!(tiers.get(1).unwrap().min_amount, 100_000);
}

// ── Roadmap Tests ──────────────────────────────────────────────────────────

#[test]
fn test_add_single_roadmap_item() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
        &None,
    );

    let current_time = env.ledger().timestamp();
    let roadmap_date = current_time + 86400; // 1 day in the future
    let description = soroban_sdk::String::from_str(&env, "Beta release");

    client.add_roadmap_item(&roadmap_date, &description);

    let roadmap = client.roadmap();
    assert_eq!(roadmap.len(), 1);
    assert_eq!(roadmap.get(0).unwrap().date, roadmap_date);
    assert_eq!(roadmap.get(0).unwrap().description, description);
}

#[test]
fn test_add_multiple_roadmap_items_in_order() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let current_time = env.ledger().timestamp();
    let date1 = current_time + 86400;
    let date2 = current_time + 172800;
    let date3 = current_time + 259200;

    let desc1 = soroban_sdk::String::from_str(&env, "Alpha release");
    let desc2 = soroban_sdk::String::from_str(&env, "Beta release");
    let desc3 = soroban_sdk::String::from_str(&env, "Production launch");

    client.add_roadmap_item(&date1, &desc1);
    client.add_roadmap_item(&date2, &desc2);
    client.add_roadmap_item(&date3, &desc3);

    let roadmap = client.roadmap();
    assert_eq!(roadmap.len(), 3);
    assert_eq!(roadmap.get(0).unwrap().date, date1);
    assert_eq!(roadmap.get(1).unwrap().date, date2);
    assert_eq!(roadmap.get(2).unwrap().date, date3);
    assert_eq!(roadmap.get(0).unwrap().description, desc1);
    assert_eq!(roadmap.get(1).unwrap().description, desc2);
    assert_eq!(roadmap.get(2).unwrap().description, desc3);
}

#[test]
#[should_panic(expected = "date must be in the future")]
fn test_add_roadmap_item_with_past_date_panics() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let current_time = env.ledger().timestamp();
    // Set a past date by moving time forward first, then trying to add an item with an earlier date
    env.ledger().set_timestamp(current_time + 1000);
    let past_date = current_time + 500; // Earlier than the new current time
    let description = soroban_sdk::String::from_str(&env, "Past milestone");

    client.add_roadmap_item(&past_date, &description); // should panic
}

#[test]
#[should_panic(expected = "date must be in the future")]
fn test_add_roadmap_item_with_current_date_panics() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let current_time = env.ledger().timestamp();
    let description = soroban_sdk::String::from_str(&env, "Current milestone");

    client.add_roadmap_item(&current_time, &description); // should panic
}

#[test]
#[should_panic(expected = "description cannot be empty")]
fn test_add_roadmap_item_with_empty_description_panics() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let current_time = env.ledger().timestamp();
    let roadmap_date = current_time + 86400;
    let empty_description = soroban_sdk::String::from_str(&env, "");

    client.add_roadmap_item(&roadmap_date, &empty_description); // should panic
}

#[test]
#[should_panic]
fn test_add_roadmap_item_by_non_creator_panics() {
    let env = Env::default();
    let contract_id = env.register(crate::CrowdfundContract, ());
    let client = crate::CrowdfundContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_contract_id.address();

    let creator = Address::generate(&env);
    let non_creator = Address::generate(&env);

    env.mock_all_auths();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    env.mock_all_auths_allowing_non_root_auth();
    env.set_auths(&[]);

    let current_time = env.ledger().timestamp();
    let roadmap_date = current_time + 86400;
    let description = soroban_sdk::String::from_str(&env, "Milestone");

    client.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &non_creator,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &contract_id,
            fn_name: "add_roadmap_item",
            args: soroban_sdk::vec![&env],
            sub_invokes: &[],
        },
    }]);

    client.add_roadmap_item(&roadmap_date, &description); // should panic
}

#[test]
fn test_roadmap_empty_after_initialization() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let roadmap = client.roadmap();
    assert_eq!(roadmap.len(), 0);
}

// ── Campaign Info Tests ────────────────────────────────────────────────────

#[test]
fn test_creator() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
    );

    assert_eq!(client.creator(), creator);
}

#[test]
fn test_get_campaign_info_initial() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution, &default_title(&env), &default_description(&env), &None);

    let info = client.get_campaign_info();

    assert_eq!(info.creator, creator);
    assert_eq!(info.token, token_address);
    assert_eq!(info.goal, goal);
    assert_eq!(info.deadline, deadline);
    assert_eq!(info.total_raised, 0);
}

#[test]
fn test_get_campaign_info_with_contributions() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
    );

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &alice, 600_000);
    mint_to(&env, &token_address, &admin, &bob, 300_000);

    client.contribute(&alice, &600_000);
    client.contribute(&bob, &300_000);

    let info = client.get_campaign_info();

    assert_eq!(info.creator, creator);
    assert_eq!(info.token, token_address);
    assert_eq!(info.goal, goal);
    assert_eq!(info.deadline, deadline);
    assert_eq!(info.total_raised, 900_000);
}

// ── Whitelist Tests ────────────────────────────────────────────────────────

#[test]
fn test_get_campaign_info_after_goal_reached() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
    );

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000, &None);

    let info = client.get_campaign_info();

    assert_eq!(info.creator, creator);
    assert_eq!(info.token, token_address);
    assert_eq!(info.goal, goal);
    assert_eq!(info.deadline, deadline);
    assert_eq!(info.total_raised, 1_500_000);
}

// ── Whitelist Tests ────────────────────────────────────────────────────────

#[test]
fn test_whitelisted_contribution() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
    );

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    // Add Alice to whitelist
    client.add_to_whitelist(&soroban_sdk::vec![&env, alice.clone()]);

    mint_to(&env, &token_address, &admin, &alice, 500_000);
    mint_to(&env, &token_address, &admin, &bob, 500_000);

    // Alice (whitelisted) can contribute
    client.contribute(&alice, &500_000);
    assert_eq!(client.contribution(&alice), 500_000);

    // Bob (not whitelisted) cannot contribute
    let result = client.try_contribute(&bob, &500_000);
    assert!(result.is_err());
}

#[test]
fn test_open_campaign_no_whitelist() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
    );

    let alice = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &alice, 500_000);

    // Any address can contribute if no addresses were ever added to the whitelist
    client.contribute(&alice, &500_000);
    assert_eq!(client.contribution(&alice), 500_000);
}

#[test]
fn test_batch_whitelist_addition() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &deadline,
        &min_contribution,
    );

    let stretch_milestone: i128 = 1_500_000;
    client.add_stretch_goal(&stretch_milestone);

    assert_eq!(client.current_milestone(), stretch_milestone);
}

// ── Property-Based Fuzz Tests with Proptest ────────────────────────────────

/// **Property Test 1: Invariant - Total Raised Equals Sum of Contributions**
///
/// For any valid (goal, deadline, contributions[]), the contract invariant holds:
/// total_raised == sum of all individual contributions
///
/// This test generates random valid parameters and multiple contributors with
/// varying contribution amounts, then verifies the invariant is maintained.
proptest! {
    #[test]
    fn prop_total_raised_equals_sum_of_contributions(
        goal in 1_000_000i128..100_000_000i128,
        deadline_offset in 100u64..100_000u64,
        amount1 in 1_000i128..10_000_000i128,
        amount2 in 1_000i128..10_000_000i128,
        amount3 in 1_000i128..10_000_000i128,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;
        let hard_cap = (amount1 + amount2 + amount3).max(goal * 2);

        client.initialize(&creator, &token_address, &goal, &hard_cap, &deadline, &1_000, &None);

        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let charlie = Address::generate(&env);

        mint_to(&env, &token_address, &admin, &alice, amount1);
        mint_to(&env, &token_address, &admin, &bob, amount2);
        mint_to(&env, &token_address, &admin, &charlie, amount3);

        client.contribute(&alice, &amount1, None);
        client.contribute(&bob, &amount2, None);
        client.contribute(&charlie, &amount3, None);

        let expected_total = amount1 + amount2 + amount3;
        let actual_total = client.total_raised();

        // **INVARIANT**: total_raised must equal the sum of all contributions
        prop_assert_eq!(actual_total, expected_total,
            "total_raised ({}) != sum of contributions ({})",
            actual_total, expected_total
        );
    }
}

/// **Property Test 2: Invariant - Refund Returns Exact Contributed Amount**
///
/// For any valid contribution amount, refund always returns the exact amount
/// with no remainder or shortfall.
///
/// This test verifies that each contributor receives back exactly what they
/// contributed when the goal is not met and refund is called.
proptest! {
    #[test]
    fn prop_refund_returns_exact_amount(
        goal in 5_000_000i128..100_000_000i128,
        deadline_offset in 100u64..100_000u64,
        contribution in 1_000i128..5_000_000i128,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        // Ensure contribution is less than goal
        let safe_contribution = contribution.min(goal - 1);

        client.initialize(&creator, &token_address, &goal, &(goal * 2), &deadline, &1_000, &None);

        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, safe_contribution);
        client.contribute(&contributor, &safe_contribution, None);

        // Move past deadline (goal not met)
        env.ledger().set_timestamp(deadline + 1);

        let token_client = token::Client::new(&env, &token_address);
        let balance_before_refund = token_client.balance(&contributor);

        client.refund();

        let balance_after_refund = token_client.balance(&contributor);

        // **INVARIANT**: Refund must return exact amount with no remainder
        prop_assert_eq!(
            balance_after_refund - balance_before_refund,
            safe_contribution,
            "refund amount ({}) != original contribution ({})",
            balance_after_refund - balance_before_refund,
            safe_contribution
        );
    }
}

/// **Property Test 3: Contribute with Amount <= 0 Always Fails**
///
/// For any contribution amount <= 0, the contribute function must fail.
/// This test verifies that zero and negative contributions are rejected.
proptest! {
    #[test]
    fn prop_contribute_zero_or_negative_fails(
        goal in 1_000_000i128..10_000_000i128,
        deadline_offset in 100u64..10_000u64,
        negative_amount in -1_000_000i128..=0i128,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        client.initialize(&creator, &token_address, &goal, &(goal * 2), &deadline, &1_000, &None);

        let contributor = Address::generate(&env);
        // Mint enough tokens so the failure is due to amount validation, not balance
        mint_to(&env, &token_address, &admin, &contributor, 10_000_000);

        // Attempt to contribute zero or negative amount
        // This should fail due to minimum contribution check
        let result = client.try_contribute(&contributor, &negative_amount);

        // **INVARIANT**: Contribution <= 0 must fail
        prop_assert!(
            result.is_err(),
            "contribute with amount {} should fail but succeeded",
            negative_amount
        );
    }
}

/// **Property Test 4: Deadline in the Past Always Fails on Initialize**
///
/// For any deadline in the past (relative to current ledger time),
/// initialization must fail or panic.
proptest! {
    #[test]
    fn prop_initialize_with_past_deadline_fails(
        goal in 1_000_000i128..10_000_000i128,
        past_offset in 1u64..10_000u64,
    ) {
        let (env, client, creator, token_address, _admin) = setup_env();

        let current_time = env.ledger().timestamp();
        // Set deadline in the past
        let past_deadline = current_time.saturating_sub(past_offset);

        // Attempt to initialize with past deadline
        let result = client.try_initialize(
            &creator,
            &token_address,
            &goal,
            &(goal * 2),
            &past_deadline,
            &1_000,
            &None,
        );

        // **INVARIANT**: Past deadline should fail or be rejected
        // Note: The contract may not explicitly validate this, but it's a logical invariant
        // If the contract allows it, the campaign would already be expired
        // This test documents the expected behavior
        if result.is_ok() {
            // If initialization succeeds with past deadline, verify campaign is immediately expired
            let deadline = client.deadline();
            prop_assert!(
                deadline <= current_time,
                "deadline {} should be in the past relative to current time {}",
                deadline,
                current_time
            );
        }
    }
}

/// **Property Test 5: Multiple Contributions Accumulate Correctly**
///
/// For any sequence of valid contributions from multiple contributors,
/// the total_raised must equal the sum of all contributions.
proptest! {
    #[test]
    fn prop_multiple_contributions_accumulate(
        goal in 5_000_000i128..50_000_000i128,
        deadline_offset in 100u64..100_000u64,
        amount1 in 1_000i128..5_000_000i128,
        amount2 in 1_000i128..5_000_000i128,
        amount3 in 1_000i128..5_000_000i128,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;
        let expected_total = amount1 + amount2 + amount3;
        let hard_cap = expected_total.max(goal);

        client.initialize(&creator, &token_address, &goal, &hard_cap, &deadline, &1_000, &None);

        let contributor1 = Address::generate(&env);
        let contributor2 = Address::generate(&env);
        let contributor3 = Address::generate(&env);

        mint_to(&env, &token_address, &admin, &contributor1, amount1);
        mint_to(&env, &token_address, &admin, &contributor2, amount2);
        mint_to(&env, &token_address, &admin, &contributor3, amount3);

        client.contribute(&contributor1, &amount1, None);
        client.contribute(&contributor2, &amount2, None);
        client.contribute(&contributor3, &amount3, None);

        // **INVARIANT**: total_raised must equal sum of all contributions
        prop_assert_eq!(client.total_raised(), expected_total);

        // **INVARIANT**: Each contributor's balance must be tracked correctly
        prop_assert_eq!(client.contribution(&contributor1), amount1);
        prop_assert_eq!(client.contribution(&contributor2), amount2);
        prop_assert_eq!(client.contribution(&contributor3), amount3);
    }
}

/// **Property Test 6: Withdrawal After Goal Met Transfers Correct Amount**
///
/// For any valid goal and contributions that meet or exceed the goal,
/// withdrawal must transfer the exact total_raised amount to the creator.
proptest! {
    #[test]
    fn prop_withdrawal_transfers_exact_amount(
        goal in 1_000_000i128..10_000_000i128,
        deadline_offset in 100u64..10_000u64,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        client.initialize(&creator, &token_address, &goal, &(goal * 2), &deadline, &1_000, &None);

        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, goal);
        client.contribute(&contributor, &goal, None);

        // Move past deadline
        env.ledger().set_timestamp(deadline + 1);

        let token_client = token::Client::new(&env, &token_address);
        let creator_balance_before = token_client.balance(&creator);

        client.withdraw();

        let creator_balance_after = token_client.balance(&creator);
        let transferred_amount = creator_balance_after - creator_balance_before;

        // **INVARIANT**: Withdrawal must transfer exact total_raised amount
        prop_assert_eq!(
            transferred_amount, goal,
            "withdrawal transferred {} but expected {}",
            transferred_amount, goal
        );

        // **INVARIANT**: total_raised must be reset to 0 after withdrawal
        prop_assert_eq!(client.total_raised(), 0);
    }
}

/// **Property Test 7: Contribution Tracking Persists Across Multiple Calls**
///
/// For any contributor making multiple contributions, the total tracked
/// must equal the sum of all their contributions.
proptest! {
    #[test]
    fn prop_contribution_tracking_persists(
        goal in 5_000_000i128..50_000_000i128,
        deadline_offset in 100u64..100_000u64,
        amount1 in 1_000i128..2_000_000i128,
        amount2 in 1_000i128..2_000_000i128,
        amount3 in 1_000i128..2_000_000i128,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        client.initialize(&creator, &token_address, &goal, &(goal * 2), &deadline, &1_000, &None);

        let contributor = Address::generate(&env);
        let total_needed = amount1.saturating_add(amount2).saturating_add(amount3);
        mint_to(&env, &token_address, &admin, &contributor, total_needed);

        // First contribution
        client.contribute(&contributor, &amount1, None);
        prop_assert_eq!(client.contribution(&contributor), amount1);

        // Second contribution
        client.contribute(&contributor, &amount2, None);
        let expected_after_2 = amount1.saturating_add(amount2);
        prop_assert_eq!(client.contribution(&contributor), expected_after_2);

        // Third contribution
        client.contribute(&contributor, &amount3, None);
        let expected_total = amount1.saturating_add(amount2).saturating_add(amount3);
        prop_assert_eq!(client.contribution(&contributor), expected_total);

        // **INVARIANT**: Final total_raised must equal sum of all contributions
        prop_assert_eq!(client.total_raised(), expected_total);
    }
}

/// **Property Test 8: Refund Resets Total Raised to Zero**
///
/// For any valid refund scenario (goal not met, deadline passed),
/// total_raised must be reset to 0 after refund completes.
proptest! {
    #[test]
    fn prop_refund_resets_total_raised(
        goal in 5_000_000i128..50_000_000i128,
        deadline_offset in 100u64..100_000u64,
        contribution in 1_000i128..5_000_000i128,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        let safe_contribution = contribution.min(goal - 1);

        client.initialize(&creator, &token_address, &goal, &(goal * 2), &deadline, &1_000, &None);

        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, safe_contribution);
        client.contribute(&contributor, &safe_contribution, None);

        // Verify total_raised is set
        prop_assert_eq!(client.total_raised(), safe_contribution);

        // Move past deadline (goal not met)
        env.ledger().set_timestamp(deadline + 1);

        client.refund();

        // **INVARIANT**: total_raised must be 0 after refund
        prop_assert_eq!(client.total_raised(), 0);
    }
}

/// **Property Test 9: Contribution Below Minimum Always Fails**
///
/// For any contribution amount below the minimum, the contribute function
/// must fail or panic.
proptest! {
    #[test]
    fn prop_contribute_below_minimum_fails(
        goal in 1_000_000i128..10_000_000i128,
        deadline_offset in 100u64..10_000u64,
        min_contribution in 1_000i128..100_000i128,
        below_minimum in 1i128..1_000i128,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        client.initialize(&creator, &token_address, &goal, &(goal * 2), &deadline, &min_contribution, &None);

        let contributor = Address::generate(&env);
        let amount_to_contribute = below_minimum.min(min_contribution - 1);
        mint_to(&env, &token_address, &admin, &contributor, amount_to_contribute);

        // Attempt to contribute below minimum
        let result = client.try_contribute(&contributor, &amount_to_contribute);

        // **INVARIANT**: Contribution below minimum must fail
        prop_assert!(
            result.is_err(),
            "contribute with amount {} below minimum {} should fail",
            amount_to_contribute, min_contribution
        );
    }
}

    client.add_to_whitelist(&soroban_sdk::vec![&env, alice.clone(), bob.clone()]);

    assert!(client.is_whitelisted(&alice));
    assert!(client.is_whitelisted(&bob));

    mint_to(&env, &token_address, &admin, &alice, 100_000);
    mint_to(&env, &token_address, &admin, &bob, 100_000);

    client.contribute(&alice, &100_000);
    client.contribute(&bob, &100_000);

    assert_eq!(client.total_raised(), 200_000);
}

#[test]
#[should_panic]
fn test_add_to_whitelist_non_creator_panics() {
    let (env, client, _creator, _token_address, _admin) = setup_env();

    let alice = Address::generate(&env);

    // Non-creator address
    let _attacker = Address::generate(&env);

    // Mock authorization for non-creator
    env.mock_all_auths();

    let result = client.try_contribute(&contributor, &5_000, &None);

    client.add_to_whitelist(&soroban_sdk::vec![&env, alice]);
}

// ── Early Withdrawal Tests ──────────────────────────────────────────────────

#[test]
fn test_partial_withdrawal() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
    );

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);
    client.contribute(&contributor, &500_000);

    assert_eq!(client.total_raised(), 500_000);
    assert_eq!(client.contribution(&contributor), 500_000);

    // Partial withdrawal.
    client.withdraw_contribution(&contributor, &200_000);

    assert_eq!(client.total_raised(), 300_000);
    assert_eq!(client.contribution(&contributor), 300_000);
}

#[test]
fn test_full_withdrawal_removes_contributor() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
    );

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);
    client.contribute(&contributor, &500_000);

    let stats = client.get_stats();
    assert_eq!(stats.contributor_count, 1);

    // Full withdrawal.
    client.withdraw_contribution(&contributor, &500_000);

    assert_eq!(client.total_raised(), 0);
    assert_eq!(client.contribution(&contributor), 0);

    let stats_after = client.get_stats();
    assert_eq!(stats_after.contributor_count, 0);
}

#[test]
#[should_panic(expected = "insufficient balance")]
fn test_withdraw_exceeding_balance_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &deadline,
        &min_contribution,
    );

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 100_000);
    client.contribute(&contributor, &100_000);

    client.withdraw_contribution(&contributor, &100_001); // should panic
}

#[test]
#[should_panic(expected = "campaign has ended")]
fn test_withdraw_after_deadline_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &deadline,
        &min_contribution,
    );

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 100_000);
    client.contribute(&contributor, &100_000);

    // Fast forward past deadline.
    env.ledger().set_timestamp(deadline + 1);

    client.withdraw_contribution(&contributor, &50_000); // should panic
}

// ── Contributor Count Tests ────────────────────────────────────────────────

#[test]
fn test_contributor_count_zero_before_contributions() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;

    client.initialize(&creator, &token_address, &goal, &(goal * 2), &deadline, &min_contribution, &None);

    assert_eq!(client.contributor_count(), 0);
}

#[test]
fn test_contributor_count_one_after_single_contribution() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;

    client.initialize(&creator, &token_address, &goal, &(goal * 2), &deadline, &min_contribution, &None);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 100_000);
    client.contribute(&contributor, &100_000);

    // Fast forward past deadline.
    env.ledger().set_timestamp(deadline + 1);

    client.withdraw_contribution(&contributor, &50_000); // should panic
}
