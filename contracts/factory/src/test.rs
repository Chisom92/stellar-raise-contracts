use crate::{FactoryContract, FactoryContractClient};
use soroban_sdk::Env;

#[test]
fn test_empty_registry() {
    let env = Env::default();

    let factory_id = env.register(FactoryContract, ());
    let factory = FactoryContractClient::new(&env, &factory_id);

    let campaigns = factory.campaigns();
    assert_eq!(campaigns.len(), 0);
    assert_eq!(factory.campaign_count(), 0);
}
