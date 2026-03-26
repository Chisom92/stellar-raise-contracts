//! Comprehensive tests for the ProptestGeneratorBoundary contract.
//!
//! @title   ProptestGeneratorBoundary Tests
//! @notice  Validates correct return of boundary constants and logic for clamping/validation.
//! @dev     Includes both unit tests and property-based tests for boundary safety.

#[cfg(test)]
mod tests {
    use soroban_sdk::{Env, Symbol};
    use proptest::prelude::*;
    use crate::proptest_generator_boundary::{
        ProptestGeneratorBoundary, ProptestGeneratorBoundaryClient,
        DEADLINE_OFFSET_MIN, DEADLINE_OFFSET_MAX, GOAL_MIN, GOAL_MAX,
    };

    /// Setup a fresh test environment with the boundary contract registered.
    fn setup() -> (Env, ProptestGeneratorBoundaryClient<'static>) {
        let env = Env::default();
        let contract_id = env.register(ProptestGeneratorBoundary, ());
        let client = ProptestGeneratorBoundaryClient::new(&env, &contract_id);
        (env, client)
    }

    #[test]
    fn test_constants_return_correct_values() {
        let (_env, client) = setup();
        assert_eq!(client.deadline_offset_min(), DEADLINE_OFFSET_MIN);
        assert_eq!(client.deadline_offset_max(), DEADLINE_OFFSET_MAX);
        assert_eq!(client.goal_min(), GOAL_MIN);
        assert_eq!(client.goal_max(), GOAL_MAX);
        assert_eq!(client.min_contribution_floor(), 1);
    }

    #[test]
    fn test_is_valid_deadline_offset() {
        let (_env, client) = setup();
        assert!(client.is_valid_deadline_offset(&1_000));
        assert!(client.is_valid_deadline_offset(&500_000));
        assert!(client.is_valid_deadline_offset(&1_000_000));
        assert!(!client.is_valid_deadline_offset(&999));
        assert!(!client.is_valid_deadline_offset(&1_000_001));
    }

    #[test]
    fn test_is_valid_goal() {
        let (_env, client) = setup();
        assert!(client.is_valid_goal(&1_000));
        assert!(client.is_valid_goal(&50_000_000));
        assert!(client.is_valid_goal(&100_000_000));
        assert!(!client.is_valid_goal(&999));
        assert!(!client.is_valid_goal(&100_000_001));
    }

    #[test]
    fn test_clamp_proptest_cases() {
        let (_env, client) = setup();
        assert_eq!(client.clamp_proptest_cases(&0), 32);
        assert_eq!(client.clamp_proptest_cases(&100), 100);
        assert_eq!(client.clamp_proptest_cases(&1000), 256);
    }

    #[test]
    fn test_compute_progress_bps() {
        let (_env, client) = setup();
        assert_eq!(client.compute_progress_bps(&500, &1000), 5000);
        assert_eq!(client.compute_progress_bps(&2000, &1000), 10000);
        assert_eq!(client.compute_progress_bps(&500, &0), 0);
        assert_eq!(client.compute_progress_bps(&-100, &1000), 0);
    }

    #[test]
    fn test_log_tag() {
        let (env, client) = setup();
        assert_eq!(client.log_tag(), Symbol::new(&env, "boundary"));
    }

    // ── Property-Based Tests ──────────────────────────────────────────────────
    
    // Note: Proptest macros typically run outside the Soroban Env, 
    // but they can invoke contract methods for validation.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn prop_deadline_offset_validity(offset in DEADLINE_OFFSET_MIN..=DEADLINE_OFFSET_MAX) {
            let (_env, client) = setup();
            prop_assert!(client.is_valid_deadline_offset(&offset));
        }

        #[test]
        fn prop_goal_validity(goal in GOAL_MIN..=GOAL_MAX) {
            let (_env, client) = setup();
            prop_assert!(client.is_valid_goal(&goal));
        }

        #[test]
        fn prop_progress_bps_bounds(raised in -1000i128..=200_000_000i128, goal in GOAL_MIN..=GOAL_MAX) {
            let (_env, client) = setup();
            let bps = client.compute_progress_bps(&raised, &goal);
            prop_assert!(bps <= 10000);
        }
    }
}
