//! Comprehensive tests for the ContractStateSize contract.
//!
//! @title   ContractStateSize Tests
//! @notice  Validates each constant exposure through the contract interface and validation logic.
//! @dev     Uses soroban-sdk's test utilities to mock the environment.

#[cfg(test)]
mod tests {
    use soroban_sdk::{Env, String};
    use crate::contract_state_size::{
        ContractStateSize, ContractStateSizeClient,
        MAX_TITLE_LENGTH, MAX_DESCRIPTION_LENGTH, MAX_CONTRIBUTORS,
    };

    /// Setup a fresh test environment with the state size contract registered.
    fn setup() -> (Env, ContractStateSizeClient<'static>) {
        let env = Env::default();
        let contract_id = env.register(ContractStateSize, ());
        let client = ContractStateSizeClient::new(&env, &contract_id);
        (env, client)
    }

    #[test]
    fn test_constants_return_correct_values() {
        let (_env, client) = setup();
        assert_eq!(client.max_title_length(), MAX_TITLE_LENGTH);
        assert_eq!(client.max_description_length(), MAX_DESCRIPTION_LENGTH);
        assert_eq!(client.max_contributors(), MAX_CONTRIBUTORS);
        assert_eq!(client.max_roadmap_items(), 32);
        assert_eq!(client.max_stretch_goals(), 32);
        assert_eq!(client.max_social_links_length(), 512);
    }

    #[test]
    fn test_validate_title() {
        let (env, client) = setup();
        let valid_title = String::from_str(&env, "A valid project title");
        let too_long_title = String::from_str(&env, &"A".repeat((MAX_TITLE_LENGTH + 1) as usize));

        assert!(client.validate_title(&valid_title));
        assert!(!client.validate_title(&too_long_title));
    }

    #[test]
    fn test_validate_description() {
        let (env, client) = setup();
        let valid_desc = String::from_str(&env, "A valid project description");
        let too_long_desc = String::from_str(&env, &"A".repeat((MAX_DESCRIPTION_LENGTH + 1) as usize));

        assert!(client.validate_description(&valid_desc));
        assert!(!client.validate_description(&too_long_desc));
    }

    #[test]
    fn test_validate_metadata_aggregate() {
        let (_env, client) = setup();
        let limit = 128 + 2048 + 512;
        assert!(client.validate_metadata_aggregate(&100));
        assert!(client.validate_metadata_aggregate(&limit));
        assert!(!client.validate_metadata_aggregate(&(limit + 1)));
    }
}
