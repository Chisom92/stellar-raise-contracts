#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Vec};

#[cfg(test)]
mod test;

// ── Data Keys ───────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
#[contracttype]
pub enum Status {
    Active,
    Successful,
    Refunded,
    Cancelled,
}

#[derive(Clone)]
#[contracttype]
pub struct Contribution {
    pub amount: i128,
    pub is_early_bird: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct CampaignStats {
    pub total_raised: i128,
    pub goal: i128,
    pub progress_bps: u32,
    pub contributor_count: u32,
    pub average_contribution: i128,
    pub largest_contribution: i128,
}

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
    /// Deadline for the "Early Bird" bonus.
    EarlyBirdDeadline,
}

// ── Contract ────────────────────────────────────────────────────────────────

#[contract]
pub struct CrowdfundContract;

#[contractimpl]
impl CrowdfundContract {
    /// Initializes a new crowdfunding campaign.
    ///
    /// # Arguments
    /// * `creator`          – The campaign creator's address.
    /// * `token`            – The token contract address used for contributions.
    /// * `goal`             – The funding goal (in the token's smallest unit).
    /// * `deadline`            – The campaign deadline as a ledger timestamp.
    /// * `min_contribution`    – The minimum contribution amount.
    /// * `early_bird_deadline` – Optional custom early bird deadline.
    pub fn initialize(
        env: Env,
        creator: Address,
        token: Address,
        goal: i128,
        deadline: u64,
        min_contribution: i128,
        early_bird_deadline: Option<u64>,
    ) {
        // Prevent re-initialization.
        if env.storage().instance().has(&DataKey::Creator) {
            panic!("already initialized");
        }

        let eb_deadline = match early_bird_deadline {
            Some(eb) => {
                if eb >= deadline {
                    panic!("early bird deadline must be before campaign deadline");
                }
                eb
            }
            None => core::cmp::min(env.ledger().timestamp() + 86400, deadline.saturating_sub(1)),
        };

        creator.require_auth();

        env.storage().instance().set(&DataKey::Creator, &creator);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::Goal, &goal);
        env.storage().instance().set(&DataKey::Deadline, &deadline);
        env.storage()
            .instance()
            .set(&DataKey::MinContribution, &min_contribution);
        env.storage()
            .instance()
            .set(&DataKey::EarlyBirdDeadline, &eb_deadline);
        env.storage().instance().set(&DataKey::TotalRaised, &0i128);
        env.storage()
            .instance()
            .set(&DataKey::Status, &Status::Active);

        let empty_contributors: Vec<Address> = Vec::new(&env);
        env.storage()
            .instance()
            .set(&DataKey::Contributors, &empty_contributors);
    }

    /// Contribute tokens to the campaign.
    ///
    /// The contributor must authorize the call. Contributions are rejected
    /// after the deadline has passed.
    pub fn contribute(env: Env, contributor: Address, amount: i128) {
        contributor.require_auth();

        let min_contribution: i128 = env.storage().instance().get(&DataKey::MinContribution).unwrap();
        if amount < min_contribution {
            panic!("amount below minimum");
        }

        let deadline: u64 = env.storage().instance().get(&DataKey::Deadline).unwrap();
        if env.ledger().timestamp() > deadline {
            panic!("campaign has ended");
        }

        let token_address: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_address);

        // Transfer tokens from the contributor to this contract.
        token_client.transfer(
            &contributor,
            &env.current_contract_address(),
            &amount,
        );

        // Update the contributor's running total.
        let mut contribution: Contribution = env
            .storage()
            .instance()
            .get(&DataKey::Contribution(contributor.clone()))
            .unwrap_or(Contribution {
                amount: 0,
                is_early_bird: false,
            });

        contribution.amount += amount;

        let eb_deadline: u64 = env
            .storage()
            .instance()
            .get(&DataKey::EarlyBirdDeadline)
            .unwrap();
        if env.ledger().timestamp() <= eb_deadline {
            if !contribution.is_early_bird {
                contribution.is_early_bird = true;
                env.events().publish(
                    (env.current_contract_address(), soroban_sdk::symbol_short!("eb_pledge")),
                    contributor.clone(),
                );
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::Contribution(contributor.clone()), &contribution);

        // Update the global total raised.
        let total: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalRaised)
            .unwrap();
        env.storage()
            .instance()
            .set(&DataKey::TotalRaised, &(total + amount));

        // Track contributor address if new.
        let mut contributors: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Contributors)
            .unwrap();
        if !contributors.contains(&contributor) {
            contributors.push_back(contributor);
            env.storage()
                .instance()
                .set(&DataKey::Contributors, &contributors);
        }
    }

    /// Withdraw raised funds — only callable by the creator after the
    /// deadline, and only if the goal has been met.
    pub fn withdraw(env: Env) {
        let status: Status = env.storage().instance().get(&DataKey::Status).unwrap();
        if status != Status::Active {
            panic!("campaign is not active");
        }

        let creator: Address = env.storage().instance().get(&DataKey::Creator).unwrap();
        creator.require_auth();

        let deadline: u64 = env.storage().instance().get(&DataKey::Deadline).unwrap();
        if env.ledger().timestamp() <= deadline {
            panic!("campaign is still active");
        }

        let goal: i128 = env.storage().instance().get(&DataKey::Goal).unwrap();
        let total: i128 = env.storage().instance().get(&DataKey::TotalRaised).unwrap();
        if total < goal {
            panic!("goal not reached");
        }

        let token_address: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_address);

        token_client.transfer(&env.current_contract_address(), &creator, &total);

        env.storage().instance().set(&DataKey::TotalRaised, &0i128);
        env.storage().instance().set(&DataKey::Status, &Status::Successful);
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
            panic!("campaign is still active");
        }

        let goal: i128 = env.storage().instance().get(&DataKey::Goal).unwrap();
        let total: i128 = env.storage().instance().get(&DataKey::TotalRaised).unwrap();
        if total >= goal {
            panic!("goal was reached; use withdraw instead");
        }

        let token_address: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_address);

        let contributors: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Contributors)
            .unwrap();

        for contributor in contributors.iter() {
            let mut contribution: Contribution = env
                .storage()
                .instance()
                .get(&DataKey::Contribution(contributor.clone()))
                .unwrap_or(Contribution {
                    amount: 0,
                    is_early_bird: false,
                });
            if contribution.amount > 0 {
                token_client.transfer(
                    &env.current_contract_address(),
                    &contributor,
                    &contribution.amount,
                );
                contribution.amount = 0;
                env.storage()
                    .instance()
                    .set(&DataKey::Contribution(contributor), &contribution);
            }
        }

        env.storage().instance().set(&DataKey::TotalRaised, &0i128);
        env.storage().instance().set(&DataKey::Status, &Status::Refunded);
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
            .instance()
            .get(&DataKey::Contributors)
            .unwrap();

        for contributor in contributors.iter() {
            let mut contribution: Contribution = env
                .storage()
                .instance()
                .get(&DataKey::Contribution(contributor.clone()))
                .unwrap_or(Contribution {
                    amount: 0,
                    is_early_bird: false,
                });
            if contribution.amount > 0 {
                token_client.transfer(
                    &env.current_contract_address(),
                    &contributor,
                    &contribution.amount,
                );
                contribution.amount = 0;
                env.storage()
                    .instance()
                    .set(&DataKey::Contribution(contributor), &contribution);
            }
        }

        env.storage().instance().set(&DataKey::TotalRaised, &0i128);
        env.storage().instance().set(&DataKey::Status, &Status::Cancelled);
    }

    // ── View helpers ────────────────────────────────────────────────────

    /// Returns the total amount raised so far.
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
        let contrib: Contribution = env
            .storage()
            .instance()
            .get(&DataKey::Contribution(contributor))
            .unwrap_or(Contribution {
                amount: 0,
                is_early_bird: false,
            });
        contrib.amount
    }

    /// Returns the minimum contribution amount.
    pub fn min_contribution(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::MinContribution)
            .unwrap()
    }

    /// Returns true if the address is an early bird contributor.
    pub fn is_early_bird(env: Env, address: Address) -> bool {
        let contrib: Contribution = env
            .storage()
            .instance()
            .get(&DataKey::Contribution(address))
            .unwrap_or(Contribution {
                amount: 0,
                is_early_bird: false,
            });
        contrib.is_early_bird
    }

    /// Returns the early bird deadline.
    pub fn early_bird_deadline(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::EarlyBirdDeadline)
            .unwrap()
    }

    /// Returns comprehensive campaign statistics.
    pub fn get_stats(env: Env) -> CampaignStats {
        let total_raised: i128 = env.storage().instance().get(&DataKey::TotalRaised).unwrap_or(0);
        let goal: i128 = env.storage().instance().get(&DataKey::Goal).unwrap();
        let contributors: Vec<Address> = env.storage().instance().get(&DataKey::Contributors).unwrap();

        let progress_bps = if goal > 0 {
            let raw = (total_raised as i128 * 10_000) / goal;
            if raw > 10_000 { 10_000 } else { raw as u32 }
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
                let contrib: Contribution = env
                    .storage()
                    .instance()
                    .get(&DataKey::Contribution(contributor))
                    .unwrap_or(Contribution {
                        amount: 0,
                        is_early_bird: false,
                    });
                if contrib.amount > largest {
                    largest = contrib.amount;
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
}
