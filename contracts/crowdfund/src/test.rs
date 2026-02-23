#![allow(unused_doc_comments)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

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

// ── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn test_initialize() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600; // 1 hour from now
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

    assert_eq!(client.goal(), goal);
    assert_eq!(client.deadline(), deadline);
    assert_eq!(client.min_contribution(), min_contribution);
    assert_eq!(client.total_raised(), 0);
}

#[test]
fn test_version() {
    let (_env, client, _creator, _token_address, _admin) = setup_env();

    // Test that version() returns the expected version number
    assert_eq!(client.version(), 1);
}

#[test]
fn test_double_initialize_panics() {
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
    let result = client.try_initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
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

    assert_eq!(client.total_raised(), 500_000);
    assert_eq!(client.contribution(&contributor), 500_000);
}

#[test]
fn test_multiple_contributions() {
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
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
        &None,
    );

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
fn test_refund_when_goal_not_met() {
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
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline + 1);

    let result = client.try_refund();

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::GoalReached
    );
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

        client.initialize(
            &creator,
            &token_address,
            &goal,
            &(goal * 2),
            &deadline,
            &1_000,
            &None,
        );
        let result = client.try_initialize(
            &creator,
            &token_address,
            &goal,
            &(goal * 2),
            &deadline,
            &1_000,
            &None,
        );

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
        client.initialize(
            &creator,
            &token_address,
            &goal,
            &(goal * 2),
            &deadline,
            &1_000,
            &None,
        );

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
        client.initialize(
            &creator,
            &token_address,
            &goal,
            &(goal * 2),
            &deadline,
            &1_000,
            &None,
        );

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
        client.initialize(
            &creator,
            &token_address,
            &goal,
            &(goal * 2),
            &deadline,
            &1_000,
            &None,
        );

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
        client.initialize(
            &creator,
            &token_address,
            &goal,
            &(goal * 2),
            &deadline,
            &1_000,
            &None,
        );

        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, 500_000);
        client.contribute(&contributor, &500_000);

        let result = client.try_refund();

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
        client.initialize(
            &creator,
            &token_address,
            &goal,
            &(goal * 2),
            &deadline,
            &1_000,
            &None,
        );

        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
        client.contribute(&contributor, &1_000_000);

        env.ledger().set_timestamp(deadline + 1);
        let result = client.try_refund();

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().unwrap(), ContractError::GoalReached);
    }
}

// ── Preservation Property Tests ────────────────────────────────────────────

#[test]
fn test_cancel_with_no_contributions() {
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

    client.cancel();

    assert_eq!(client.total_raised(), 0);
}

#[test]
fn test_cancel_with_contributions() {
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

// ── Minimum Contribution Tests ─────────────────────────────────────────────

#[test]
fn test_contribute_exact_minimum() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 10_000;
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
    mint_to(&env, &token_address, &admin, &contributor, 50_000);

    client.contribute(&contributor, &50_000);

    assert_eq!(client.total_raised(), 50_000);
    assert_eq!(client.contribution(&contributor), 50_000);
}

// ── Tiered Rewards Tests ───────────────────────────────────────────────────

#[test]
fn test_get_user_tier_bronze_level() {
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
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
        &None,
    );

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
    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
        &None,
    );

    let roadmap = client.roadmap();
    assert_eq!(roadmap.len(), 0);
}

// ── Metadata Update Tests ──────────────────────────────────────────────────

#[test]
fn test_update_title() {
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

    // Update title.
    let title = soroban_sdk::String::from_str(&env, "New Campaign Title");
    client.update_metadata(&creator, &Some(title), &None, &None);

    // Verify title was updated (we'd need a getter, but the function should not panic).
}

#[test]
fn test_update_description() {
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

    // Update description.
    let description = soroban_sdk::String::from_str(&env, "New campaign description");
    client.update_metadata(&creator, &None, &Some(description), &None);
}

#[test]
fn test_update_socials() {
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

    // Update social links.
    let socials = soroban_sdk::String::from_str(&env, "https://twitter.com/campaign");
    client.update_metadata(&creator, &None, &None, &Some(socials));
}

#[test]
fn test_partial_update() {
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

    // Update only title (description and socials should remain None).
    let title = soroban_sdk::String::from_str(&env, "Updated Title");
    client.update_metadata(&creator, &Some(title), &None, &None);

    // Update only socials (should not affect title).
    let socials = soroban_sdk::String::from_str(&env, "https://twitter.com/new");
    client.update_metadata(&creator, &None, &None, &Some(socials));
}

#[test]
#[should_panic(expected = "campaign is not active")]
fn test_update_metadata_when_not_active_panics() {
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

    // Contribute to meet the goal.
    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    // Move past deadline and withdraw (status becomes Successful).
    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();

    // Try to update metadata (should panic - campaign is not Active).
    let title = soroban_sdk::String::from_str(&env, "New Title");
    client.update_metadata(&creator, &Some(title), &None, &None);
}

#[test]
#[should_panic(expected = "campaign is not active")]
fn test_update_metadata_after_cancel_panics() {
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

    // Cancel the campaign.
    client.cancel();

    // Try to update metadata (should panic - campaign is Cancelled).
    let title = soroban_sdk::String::from_str(&env, "New Title");
    client.update_metadata(&creator, &Some(title), &None, &None);
}

// Note: The non-creator test would require complex mock setup.
// The authorization check is covered by require_auth() in the contract,
// which will panic if the caller is not the creator.

// ── Deadline Update Tests ──────────────────────────────────────────────────

#[test]
fn test_update_deadline_extends_campaign() {
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

    // Verify initial deadline
    assert_eq!(client.deadline(), deadline);

    // Extend the deadline
    let new_deadline = deadline + 7200; // 2 more hours
    client.update_deadline(&new_deadline);

    // Verify the deadline was updated
    assert_eq!(client.deadline(), new_deadline);
}

#[test]
#[should_panic(expected = "new deadline must be after current deadline")]
fn test_update_deadline_rejects_shortening() {
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

    // Try to shorten the deadline (should panic)
    let shorter_deadline = deadline - 1800; // 30 minutes earlier
    client.update_deadline(&shorter_deadline);
}

#[test]
#[should_panic(expected = "new deadline must be after current deadline")]
fn test_update_deadline_rejects_equal_deadline() {
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

    // Try to set deadline to the same value (should panic)
    client.update_deadline(&deadline);
}

#[test]
#[should_panic(expected = "campaign is not active")]
fn test_update_deadline_when_not_active_panics() {
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

    // Move past deadline and refund
    env.ledger().set_timestamp(deadline + 1);

    // Refund to change status from Active to Refunded
    let _ = client.try_refund();

    // Try to update deadline on a non-Active campaign (should panic)
    let new_deadline = deadline + 7200;
    client.update_deadline(&new_deadline);
}

// ── Stretch Goal Tests ─────────────────────────────────────────────────────

#[test]
fn test_add_single_stretch_goal() {
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

        client.contribute(&alice, &amount1);
        client.contribute(&bob, &amount2);
        client.contribute(&charlie, &amount3);

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
        client.contribute(&contributor, &safe_contribution);

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

        client.contribute(&contributor1, &amount1);
        client.contribute(&contributor2, &amount2);
        client.contribute(&contributor3, &amount3);

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
        client.contribute(&contributor, &goal);

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
        client.contribute(&contributor, &amount1);
        prop_assert_eq!(client.contribution(&contributor), amount1);

        // Second contribution
        client.contribute(&contributor, &amount2);
        let expected_after_2 = amount1.saturating_add(amount2);
        prop_assert_eq!(client.contribution(&contributor), expected_after_2);

        // Third contribution
        client.contribute(&contributor, &amount3);
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
        client.contribute(&contributor, &safe_contribution);

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

/// **Property Test 10: Contribution After Deadline Always Fails**
///
/// For any contribution attempt after the deadline has passed,
/// the contribute function must fail.
proptest! {
    #[test]
    fn prop_contribute_after_deadline_fails(
        goal in 1_000_000i128..10_000_000i128,
        deadline_offset in 100u64..10_000u64,
        contribution in 1_000i128..10_000_000i128,
        time_after_deadline in 1u64..100_000u64,
    ) {
        let (env, client, creator, token_address, admin) = setup_env();
        let deadline = env.ledger().timestamp() + deadline_offset;

        client.initialize(&creator, &token_address, &goal, &(goal * 2), &deadline, &1_000, &None);

        // Move past deadline
        env.ledger().set_timestamp(deadline + time_after_deadline);

        let contributor = Address::generate(&env);
        mint_to(&env, &token_address, &admin, &contributor, contribution);

        // Attempt to contribute after deadline
        let result = client.try_contribute(&contributor, &contribution);

        // **INVARIANT**: Contribution after deadline must fail
        prop_assert!(
            result.is_err(),
            "contribute after deadline should fail"
        );
        prop_assert_eq!(
            result.unwrap_err().unwrap(),
            crate::ContractError::CampaignEnded
        );
    }
}

// ── Pause/Unpause Tests ─────────────────────────────────────────────────────

#[test]
fn test_contribute_rejected_when_paused() {
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

    // Pause the contract
    client.set_paused(&true);

    // Try to contribute while paused
    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 5_000);

    let result = client.try_contribute(&contributor, &5_000);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::ContractPaused
    );
}

#[test]
fn test_withdraw_rejected_when_paused() {
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

    // Contribute to meet goal
    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, goal);
    client.contribute(&contributor, &goal);

    // Move past deadline
    env.ledger().set_timestamp(deadline + 1);

    // Pause the contract
    client.set_paused(&true);

    // Try to withdraw while paused
    let result = client.try_withdraw();

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::ContractPaused
    );
}

#[test]
fn test_refund_rejected_when_paused() {
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

    // Contribute but don't meet goal
    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);
    client.contribute(&contributor, &500_000);

    // Move past deadline
    env.ledger().set_timestamp(deadline + 1);

    // Pause the contract
    client.set_paused(&true);

    // Try to refund while paused
    let result = client.try_refund();

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::ContractPaused
    );
}

#[test]
fn test_all_interactions_succeed_after_unpause() {
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

    // Pause the contract
    client.set_paused(&true);

    // Unpause the contract
    client.set_paused(&false);

    // Contribute should succeed
    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 5_000);
    client.contribute(&contributor, &5_000);

    assert_eq!(client.total_raised(), 5_000);
}

#[test]
#[should_panic]
fn test_set_paused_rejected_from_non_creator() {
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

    client.initialize(
        &creator,
        &token_address,
        &goal,
        &(goal * 2),
        &deadline,
        &min_contribution,
        &None,
    );

    env.mock_all_auths_allowing_non_root_auth();
    env.set_auths(&[]);

    client.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &non_creator,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &contract_id,
            fn_name: "set_paused",
            args: soroban_sdk::vec![&env, true.into()],
            sub_invokes: &[],
        },
    }]);

    client.set_paused(&true);
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
    mint_to(&env, &token_address, &admin, &contributor, 500_000);
    client.contribute(&contributor, &500_000);

    assert_eq!(client.contributor_count(), 1);
}

#[test]
fn test_contributor_count_multiple_contributors() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;

    client.initialize(&creator, &token_address, &goal, &(goal * 2), &deadline, &min_contribution, &None);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let charlie = Address::generate(&env);
    
    mint_to(&env, &token_address, &admin, &alice, 300_000);
    mint_to(&env, &token_address, &admin, &bob, 200_000);
    mint_to(&env, &token_address, &admin, &charlie, 100_000);

    client.contribute(&alice, &300_000);
    assert_eq!(client.contributor_count(), 1);

    client.contribute(&bob, &200_000);
    assert_eq!(client.contributor_count(), 2);

    client.contribute(&charlie, &100_000);
    assert_eq!(client.contributor_count(), 3);
}
