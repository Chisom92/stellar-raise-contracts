// Factory contract for batch campaign initialization
// Implements Issue #68 and extends Issue #23

use soroban_sdk::{contractimpl, contracttype, BytesN, Address, Env, Symbol, String, Vec};

// Registry key for storing deployed campaigns
const REGISTRY_KEY: &str = "campaign_registry";

// The WASM hash for the crowdfund contract (should be set to the correct value in production)
const CROWDFUND_WASM_HASH: [u8; 32] = [0u8; 32]; // TODO: Replace with actual hash

#[contracttype]
pub struct BatchCreatedEvent {
    pub count: u32,
    pub addresses: Vec<Address>,
}
#[derive(Clone)]
pub struct CampaignConfig {
    pub creator: Address,
    pub token: Address,
    pub goal: i128,
    pub deadline: u64,
    pub title: String,
    pub description: String,
}

#[derive(Clone)]
pub struct FactoryContract;

#[derive(Debug, PartialEq)]
pub enum ContractError {
    EmptyBatch,
    InvalidConfig { index: usize },
    // ...other errors
}

#[contractimpl]
impl FactoryContract {
    pub fn create_campaigns_batch(
        env: Env,
        configs: Vec<CampaignConfig>,
    ) -> Result<Vec<Address>, ContractError> {
        if configs.is_empty() {
            return Err(ContractError::EmptyBatch);
        }
        let mut deployed = Vec::new(&env);
        // Validate all configs first
        for (i, config) in configs.iter().enumerate() {
            if config.goal <= 0 || config.title.is_empty() || config.description.is_empty() {
                return Err(ContractError::InvalidConfig { index: i });
            }
        }
        // Deploy and initialize all campaigns
        for config in configs.iter() {
            let campaign_addr = deploy_and_init_campaign(&env, config);
            deployed.push_back(campaign_addr);
        }
        // Store all deployed addresses in the factory registry
        let mut registry: Vec<Address> = env
            .storage()
            .persistent()
            .get(&REGISTRY_KEY.into())
            .unwrap_or(Vec::new(&env));
        for addr in deployed.iter() {
            registry.push_back(addr.clone());
        }
        env.storage().persistent().set(&REGISTRY_KEY.into(), &registry);
        // Emit batch_campaigns_created event
        let event = BatchCreatedEvent {
            count: deployed.len() as u32,
            addresses: deployed.clone(),
        };
        env.events().publish(("factory", "batch_campaigns_created"), event);
        Ok(deployed)
    }
}

fn deploy_and_init_campaign(env: &Env, config: &CampaignConfig) -> Address {
    // Deploy the crowdfund contract
    let wasm_hash = BytesN::from_array(env, &CROWDFUND_WASM_HASH);
    let campaign_addr = env
        .deployer()
        .with_current_contract(env.current_contract_address())
        .deploy_contract(wasm_hash);
    // Call initialize on the deployed contract
    // NOTE: Hard cap, min_contribution, platform_config are set to defaults for this example
    let hard_cap = config.goal;
    let min_contribution = 1i128;
    let platform_config: Option<()> = None;
    env.invoke_contract(
        &campaign_addr,
        &Symbol::short("initialize"),
        (
            config.creator.clone(),
            config.token.clone(),
            config.goal,
            hard_cap,
            config.deadline,
            min_contribution,
            platform_config,
        ),
    );
    campaign_addr
}
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

    #[test]
    fn test_batch_deploys_campaigns() {
        let env = Env::default();
        let configs = Vec::from_array(
            &env,
            [
                CampaignConfig {
                    creator: Address::random(&env),
                    token: Address::random(&env),
                    goal: 1000,
                    deadline: 123456,
                    title: "Campaign 1".to_string(),
                    description: "Desc 1".to_string(),
                },
                CampaignConfig {
                    creator: Address::random(&env),
                    token: Address::random(&env),
                    goal: 2000,
                    deadline: 223456,
                    title: "Campaign 2".to_string(),
                    description: "Desc 2".to_string(),
                },
                CampaignConfig {
                    creator: Address::random(&env),
                    token: Address::random(&env),
                    goal: 3000,
                    deadline: 323456,
                    title: "Campaign 3".to_string(),
                    description: "Desc 3".to_string(),
                },
            ],
        );
        let result = FactoryContract::create_campaigns_batch(env.clone(), configs.clone());
        assert!(result.is_ok());
        let deployed = result.unwrap();
        assert_eq!(deployed.len(), 3);
        // TODO: Check registry and returned addresses
    }

    #[test]
    fn test_empty_batch_rejected() {
        let env = Env::default();
        let configs = Vec::new(&env);
        let result = FactoryContract::create_campaigns_batch(env, configs);
        assert_eq!(result, Err(ContractError::EmptyBatch));
    }

    #[test]
    fn test_invalid_config_rolls_back_batch() {
        let env = Env::default();
        let configs = Vec::from_array(
            &env,
            [
                CampaignConfig {
                    creator: Address::random(&env),
                    token: Address::random(&env),
                    goal: 1000,
                    deadline: 123456,
                    title: "Valid".to_string(),
                    description: "Valid".to_string(),
                },
                CampaignConfig {
                    creator: Address::random(&env),
                    token: Address::random(&env),
                    goal: -1, // Invalid goal
                    deadline: 223456,
                    title: "Invalid".to_string(),
                    description: "Invalid".to_string(),
                },
            ],
        );
        let result = FactoryContract::create_campaigns_batch(env, configs);
        assert_eq!(result, Err(ContractError::InvalidConfig { index: 1 }));
    }
}

// TODO: Add tests for batch deployment and error handling
