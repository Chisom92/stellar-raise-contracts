#![no_std]
#![allow(missing_docs)]

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, String, Vec};

#[cfg(test)]
mod test;

// ── Data Types ──────────────────────────────────────────────────────────────

/// Represents the campaign status.
#[derive(Clone, PartialEq)]
#[contracttype]
pub enum Status {
    /// The campaign is currently active and accepting contributions.
    Active,
    /// The campaign was successful and goal was met.
    Successful,
    /// The campaign was refunded because goal was not met.
    Refunded,
    /// The campaign was cancelled by the creator.
    Cancelled,
}

/// Campaign statistics for the get_stats view.
#[derive(Clone)]
#[contracttype]
pub struct RoadmapItem {
    pub date: u64,
    pub description: String,
}

/// Represents all storage keys used by the crowdfund contract.
#[derive(Clone)]
#[contracttype]
pub struct CampaignStats {
    /// Total amount raised so far.
    pub total_raised: i128,
    /// The funding goal.
    pub goal: i128,
    /// Progress towards goal in basis points (10000 = 100%).
    pub progress_bps: u32,
    /// Number of contributors.
    pub contributor_count: u32,
    /// Average contribution amount.
    pub average_contribution: i128,
    /// Largest contribution amount.
    pub largest_contribution: i128,
}

/// Represents all storage keys used by the crowdfund contract.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// The address of the campaign creator.
    Creator,
    /// The token used for contributions (e.g. USDC).
    Token,
    /// The funding goal in the token's smallest unit.
    Goal,
    /// The deadline as a ledger timestamp.
    Deadline,
    /// Total amount raised so far.
    TotalRaised,
    /// Individual contribution by address.
    Contribution(Address),
    /// List of all contributor addresses.
    Contributors,
    /// Campaign status (Active, Successful, Refunded).
    Status,
    /// Minimum contribution amount.
    MinContribution,
    /// List of roadmap items with dates and descriptions.
    Roadmap,
    /// The address authorized to upgrade the contract.
    Admin,
}

// ── Contract Error ──────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    AlreadyInitialized = 1,
    CampaignEnded = 2,
    CampaignStillActive = 3,
    GoalNotReached = 4,
    GoalReached = 5,
}

// ── Contract ────────────────────────────────────────────────────────────────

/// The main crowdfund contract implementation.
#[contract]
pub struct CrowdfundContract;

#[contractimpl]
impl CrowdfundContract {
    /// Initializes a new crowdfunding campaign.
    ///
    /// # Arguments
    /// * `creator`            – The campaign creator's address.
    /// * `token`              – The token contract address used for contributions.
    /// * `goal`               – The funding goal (in the token's smallest unit).
    /// * `deadline`           – The campaign deadline as a ledger timestamp.
    /// * `min_contribution`   – The minimum contribution amount.
    /// * `platform_config`    – Optional platform configuration (address and fee in basis points).
    ///
    /// # Panics
    /// * If already initialized.
    /// * If platform fee exceeds 10,000 (100%).
    pub fn initialize(
        env: Env,
        creator: Address,
        token: Address,
        goal: i128,
        deadline: u64,
        min_contribution: i128,
        platform_config: Option<PlatformConfig>,
    ) {
        // Prevent re-initialization.
        if env.storage().instance().has(&DataKey::Creator) {
            return Err(ContractError::AlreadyInitialized);
        }

        creator.require_auth();

        // Validate platform fee if provided.
        if let Some(ref config) = platform_config {
            if config.fee_bps > 10_000 {
                panic!("platform fee cannot exceed 100%");
            }
        }

        env.storage().instance().set(&DataKey::Creator, &creator);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::Goal, &goal);
        env.storage().instance().set(&DataKey::Deadline, &deadline);
        env.storage()
            .instance()
            .set(&DataKey::MinContribution, &min_contribution);
        env.storage().instance().set(&DataKey::TotalRaised, &0i128);

        let empty_contributors: Vec<Address> = Vec::new(&env);
        env.storage()
            .persistent()
            .set(&DataKey::Contributors, &empty_contributors);

        let empty_roadmap: Vec<RoadmapItem> = Vec::new(&env);
        env.storage()
            .instance()
            .set(&DataKey::Roadmap, &empty_roadmap);
    }

    /// Contribute tokens to the campaign.
    ///
    /// The contributor must authorize the call. Contributions are rejected
    /// after the deadline has passed.
    pub fn contribute(env: Env, contributor: Address, amount: i128) -> Result<(), ContractError> {
        contributor.require_auth();

        let min_contribution: i128 = env
            .storage()
            .instance()
            .get(&DataKey::MinContribution)
            .unwrap();
        if amount < min_contribution {
            panic!("amount below minimum");
        }

        let deadline: u64 = env.storage().instance().get(&DataKey::Deadline).unwrap();
        if env.ledger().timestamp() > deadline {
            return Err(ContractError::CampaignEnded);
        }

        let token_address: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_address);

        // Transfer tokens from the contributor to this contract.
        token_client.transfer(&contributor, &env.current_contract_address(), &amount);

        // Update the contributor's running total.
        let contribution_key = DataKey::Contribution(contributor.clone());
        let prev: i128 = env
            .storage()
            .persistent()
            .get(&contribution_key)
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&contribution_key, &(prev + amount));
        env.storage()
            .persistent()
            .extend_ttl(&contribution_key, 100, 100);

        // Update the global total raised.
        let total: i128 = env.storage().instance().get(&DataKey::TotalRaised).unwrap();
        env.storage()
            .instance()
            .set(&DataKey::TotalRaised, &(total + amount));

        // Track contributor address if new.
        let mut contributors: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Contributors)
            .unwrap();
        if !contributors.contains(&contributor) {
            contributors.push_back(contributor.clone());
            env.storage()
                .persistent()
                .set(&DataKey::Contributors, &contributors);
            env.storage()
                .persistent()
                .extend_ttl(&DataKey::Contributors, 100, 100);
        }

        Ok(())
    }

    /// Withdraw raised funds — only callable by the creator after the
    /// deadline, and only if the goal has been met.
    ///
    /// If a platform fee is configured, deducts the fee and transfers it to
    /// the platform address, then sends the remainder to the creator.
    pub fn withdraw(env: Env) {
        let status: Status = env.storage().instance().get(&DataKey::Status).unwrap();
        if status != Status::Active {
            panic!("campaign is not active");
        }

        let creator: Address = env.storage().instance().get(&DataKey::Creator).unwrap();
        creator.require_auth();

        let deadline: u64 = env.storage().instance().get(&DataKey::Deadline).unwrap();
        if env.ledger().timestamp() <= deadline {
            return Err(ContractError::CampaignStillActive);
        }

        let goal: i128 = env.storage().instance().get(&DataKey::Goal).unwrap();
        let total: i128 = env.storage().instance().get(&DataKey::TotalRaised).unwrap();
        if total < goal {
            return Err(ContractError::GoalNotReached);
        }

        let token_address: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_address);

        // Calculate and transfer platform fee if configured.
        let platform_config: Option<PlatformConfig> =
            env.storage().instance().get(&DataKey::PlatformConfig);

        let creator_payout = if let Some(config) = platform_config {
            // Calculate fee using checked arithmetic to prevent overflow.
            let fee = total
                .checked_mul(config.fee_bps as i128)
                .expect("fee calculation overflow")
                .checked_div(10_000)
                .expect("fee division by zero");

            // Transfer fee to platform.
            token_client.transfer(&env.current_contract_address(), &config.address, &fee);

            // Emit event with fee details.
            env.events()
                .publish(("campaign", "fee_transferred"), (&config.address, fee));

            // Calculate creator payout.
            total.checked_sub(fee).expect("creator payout underflow")
        } else {
            total
        };

        // Transfer remainder to creator.
        token_client.transfer(&env.current_contract_address(), &creator, &creator_payout);

        env.storage().instance().set(&DataKey::TotalRaised, &0i128);
        env.storage().instance().set(&DataKey::Status, &Status::Successful);

        // Emit withdrawal event
        env.events().publish(
            ("campaign", "withdrawn"),
            (creator.clone(), total),
        );
    }

    /// Refund all contributors — callable by anyone after the deadline
    /// if the goal was **not** met.
    pub fn refund(env: Env) {
        let status: Status = env.storage().instance().get(&DataKey::Status).unwrap();
        if status != Status::Active {
            panic!("campaign is not active");
        }

        let deadline: u64 = env.storage().instance().get(&DataKey::Deadline).unwrap();
        if env.ledger().timestamp() <= deadline {
            return Err(ContractError::CampaignStillActive);
        }

        let goal: i128 = env.storage().instance().get(&DataKey::Goal).unwrap();
        let total: i128 = env.storage().instance().get(&DataKey::TotalRaised).unwrap();
        if total >= goal {
            return Err(ContractError::GoalReached);
        }

        let token_address: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_address);

        let contributors: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Contributors)
            .unwrap();

        for contributor in contributors.iter() {
            let contribution_key = DataKey::Contribution(contributor.clone());
            let amount: i128 = env
                .storage()
                .persistent()
                .get(&contribution_key)
                .unwrap_or(0);
            if amount > 0 {
                token_client.transfer(&env.current_contract_address(), &contributor, &amount);
                env.storage()
                    .persistent()
                    .set(&contribution_key, &0i128);
                env.storage()
                    .persistent()
                    .extend_ttl(&contribution_key, 100, 100);
            }
        }

        env.storage().instance().set(&DataKey::TotalRaised, &0i128);
        env.storage()
            .instance()
            .set(&DataKey::Status, &Status::Refunded);
    }

    /// Cancel the campaign and refund all contributors — callable only by
    /// the creator while the campaign is still Active.
    pub fn cancel(env: Env) {
        let status: Status = env.storage().instance().get(&DataKey::Status).unwrap();
        if status != Status::Active {
            panic!("campaign is not active");
        }

        let creator: Address = env.storage().instance().get(&DataKey::Creator).unwrap();
        creator.require_auth();

        let token_address: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_address);

        let contributors: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Contributors)
            .unwrap();

        for contributor in contributors.iter() {
            let contribution_key = DataKey::Contribution(contributor.clone());
            let amount: i128 = env
                .storage()
                .persistent()
                .get(&contribution_key)
                .unwrap_or(0);
            if amount > 0 {
                token_client.transfer(&env.current_contract_address(), &contributor, &amount);
                env.storage()
                    .persistent()
                    .set(&contribution_key, &0i128);
                env.storage()
                    .persistent()
                    .extend_ttl(&contribution_key, 100, 100);
            }
        }

        env.storage().instance().set(&DataKey::TotalRaised, &0i128);
        env.storage()
            .instance()
            .set(&DataKey::Status, &Status::Cancelled);
    }

    /// Upgrade the contract to a new WASM implementation — admin-only.
    ///
    /// This function allows the designated admin to upgrade the contract's WASM code
    /// without changing the contract's address or storage. The new WASM hash must be
    /// provided and the caller must be authorized as the admin.
    ///
    /// # Arguments
    /// * `new_wasm_hash` – The SHA-256 hash of the new WASM binary to deploy.
    ///
    /// # Panics
    /// * If the caller is not the admin.
    pub fn upgrade(env: Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    /// Update campaign metadata — only callable by the creator while the
    /// campaign is still Active.
    ///
    /// # Arguments
    /// * `creator`     – The campaign creator's address (for authentication).
    /// * `title`       – Optional new title (None to keep existing).
    /// * `description` – Optional new description (None to keep existing).
    /// * `socials`    – Optional new social links (None to keep existing).
    pub fn update_metadata(env: Env, creator: Address, title: Option<String>, description: Option<String>, socials: Option<String>) {
        // Check campaign is active.
        let status: Status = env.storage().instance().get(&DataKey::Status).unwrap();
        if status != Status::Active {
            panic!("campaign is not active");
        }

        // Require creator authentication and verify caller is the creator.
        let stored_creator: Address = env.storage().instance().get(&DataKey::Creator).unwrap();
        if creator != stored_creator {
            panic!("not authorized");
        }
        creator.require_auth();

        // Track which fields were updated for the event.
        let mut updated_fields: Vec<Symbol> = Vec::new(&env);

        // Update title if provided.
        if let Some(new_title) = title {
            env.storage().instance().set(&DataKey::Title, &new_title);
            updated_fields.push_back(Symbol::new(&env, "title"));
        }

        // Update description if provided.
        if let Some(new_description) = description {
            env.storage().instance().set(&DataKey::Description, &new_description);
            updated_fields.push_back(Symbol::new(&env, "description"));
        }

        // Update social links if provided.
        if let Some(new_socials) = socials {
            env.storage().instance().set(&DataKey::SocialLinks, &new_socials);
            updated_fields.push_back(Symbol::new(&env, "socials"));
        }

        // Emit metadata_updated event with the list of updated field names.
        env.events().publish((Symbol::new(&env, "campaign"), Symbol::new(&env, "metadata_updated")), updated_fields);
    }

    // ── View helpers ────────────────────────────────────────────────────

    /// Add a roadmap item to the campaign timeline.
    ///
    /// Only the creator can add roadmap items. The date must be in the future
    /// and the description must not be empty.
    pub fn add_roadmap_item(env: Env, date: u64, description: String) {
        let creator: Address = env.storage().instance().get(&DataKey::Creator).unwrap();
        creator.require_auth();

        let current_timestamp = env.ledger().timestamp();
        if date <= current_timestamp {
            panic!("date must be in the future");
        }

        if description.is_empty() {
            panic!("description cannot be empty");
        }

        let mut roadmap: Vec<RoadmapItem> = env
            .storage()
            .instance()
            .get(&DataKey::Roadmap)
            .unwrap_or_else(|| Vec::new(&env));

        let item = RoadmapItem {
            date,
            description: description.clone(),
        };

        roadmap.push_back(item.clone());
        env.storage().instance().set(&DataKey::Roadmap, &roadmap);

        env.events()
            .publish(("campaign", "roadmap_item_added"), (date, description));
    }

    /// Returns the full ordered list of roadmap items.
    pub fn roadmap(env: Env) -> Vec<RoadmapItem> {
        env.storage()
            .instance()
            .get(&DataKey::Roadmap)
            .unwrap_or_else(|| Vec::new(&env))
    }
    pub fn total_raised(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalRaised)
            .unwrap_or(0)
    }

    /// Returns the funding goal.
    pub fn goal(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::Goal).unwrap()
    }

    /// Returns the campaign deadline.
    pub fn deadline(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::Deadline).unwrap()
    }

    /// Returns the contribution of a specific address.
    pub fn contribution(env: Env, contributor: Address) -> i128 {
        let contribution_key = DataKey::Contribution(contributor);
        env.storage()
            .persistent()
            .get(&contribution_key)
            .unwrap_or(0)
    }

    /// Returns the minimum contribution amount.
    pub fn min_contribution(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::MinContribution)
            .unwrap()
    }

    /// Returns comprehensive campaign statistics.
    pub fn get_stats(env: Env) -> CampaignStats {
        let total_raised: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalRaised)
            .unwrap_or(0);
        let goal: i128 = env.storage().instance().get(&DataKey::Goal).unwrap();
        let contributors: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Contributors)
            .unwrap();

        let progress_bps = if goal > 0 {
            let raw = (total_raised * 10_000) / goal;
            if raw > 10_000 {
                10_000
            } else {
                raw as u32
            }
        } else {
            0
        };

        let contributor_count = contributors.len();
        let (average_contribution, largest_contribution) = if contributor_count == 0 {
            (0, 0)
        } else {
            let average = total_raised / contributor_count as i128;
            let mut largest = 0i128;
            for contributor in contributors.iter() {
                let amount: i128 = env
                    .storage()
                    .instance()
                    .get(&DataKey::Contribution(contributor))
                    .unwrap_or(0);
                if amount > largest {
                    largest = amount;
                }
            }
            (average, largest)
        };

        CampaignStats {
            total_raised,
            goal,
            progress_bps,
            contributor_count,
            average_contribution,
            largest_contribution,
        }
    }

    /// Returns the campaign title.
    pub fn title(env: Env) -> String {
        let empty = String::from(&String::from_slice(&env, ""));
        env.storage()
            .instance()
            .get(&DataKey::Title)
            .unwrap_or(empty)
    }

    /// Returns the campaign description.
    pub fn description(env: Env) -> String {
        let empty = String::from(&String::from_slice(&env, ""));
        env.storage()
            .instance()
            .get(&DataKey::Description)
            .unwrap_or(empty)
    }

    /// Returns the campaign social links.
    pub fn socials(env: Env) -> String {
        let empty = String::from(&String::from_slice(&env, ""));
        env.storage()
            .instance()
            .get(&DataKey::SocialLinks)
            .unwrap_or(empty)
    }
}
