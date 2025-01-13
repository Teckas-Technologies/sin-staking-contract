use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{UnorderedMap, Vector},
    env, near_bindgen, AccountId, PanicOnDefault, NearToken,
};
use near_sdk::{json_types::U128, Gas};
use serde_json::json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use near_contract_standards::fungible_token::Balance;
use near_sdk::Promise;

const DAY: u64 = 86400; // Seconds in a day
const MONTH: u64 = 30 * DAY; // Approximate seconds in a month

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct StakingRecord {
    pub staked_tokens: Balance,
    pub start_timestamp: u64,
    pub lockup_period: u64, // Lockup period in seconds
    pub claimed_rewards: Balance,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct StakerInfo {
    pub stakes: Vector<StakingRecord>,
    pub total_rewards_claimed: Balance,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FundingRecord {
    pub amount: Balance,
    pub timestamp: u64,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct RewardDistribution {
    pub total_reward_pool: Balance,
    pub last_distributed: u64, // Timestamp of last reward distribution
    pub funding_records: Vector<FundingRecord>, // Track funding history
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct StakingContract {
    pub owner: AccountId,
    pub sin_token: AccountId, // SIN token contract address
    pub stakers: UnorderedMap<AccountId, StakerInfo>,
    pub reward_distribution: RewardDistribution,
    pub staking_weight: HashMap<u64, f64>, // Map for weight calculation
}

#[near_bindgen]
impl StakingContract {
    #[init]
    pub fn new(owner: AccountId, sin_token: AccountId) -> Self {
        let mut staking_weight = HashMap::new();
        staking_weight.insert(90 * DAY, 1.0);
        staking_weight.insert(180 * DAY, 1.5);
        staking_weight.insert(270 * DAY, 2.0);
        staking_weight.insert(u64::MAX, 2.5);

        Self {
            owner,
            sin_token,
            stakers: UnorderedMap::new(b"s".to_vec()),
            reward_distribution: RewardDistribution {
                total_reward_pool: 0,
                last_distributed: env::block_timestamp(),
                funding_records: Vector::new(b"fundings".to_vec()),
            },
            staking_weight,
        }
    }

    #[payable]
    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> U128 {
        env::log_str(&format!("Received {} tokens from {}", amount.0, sender_id));

        if sender_id == self.owner {
            assert_eq!(
                env::predecessor_account_id(),
                self.sin_token,
                "Only SIN tokens are accepted for funding"
            );
            assert!(amount.0 > 0, "Funding amount must be greater than zero");
    
            // Update total reward pool
            self.reward_distribution.total_reward_pool += amount.0;
    
            // Track funding record
            self.reward_distribution.funding_records.push(&FundingRecord {
                amount: amount.0,
                timestamp: env::block_timestamp(),
            });
    
            env::log_str(&format!(
                "Reward pool funded with {} SIN tokens by {} with message {}",
                amount.0, env::predecessor_account_id(), msg
            ));
            // Return 0 to indicate all tokens were accepted
            U128(0)
        } else {
            assert_eq!(
                env::predecessor_account_id(),
                self.sin_token,
                "Only SIN tokens are accepted for staking"
            );
            assert!(amount.0 > 0, "Staking amount must be greater than zero");
             // Default lockup period of 30 days
            let lockup_days: u64 = 30;
             // Call the staking logic
            self.stake_tokens(sender_id, amount.0, lockup_days);

            env::log_str(&format!(
                "Staked {} SIN tokens by {} with message {}",
                amount.0, env::predecessor_account_id(), msg
            ));
            // Return 0 to indicate all tokens were accepted
            U128(0)
        }
       
    }

    pub fn stake_tokens(&mut self, sender_id: AccountId, amount: u128, lockup_days: u64) {
        env::log_str(&format!(
            "Staking {} tokens for {} days from {}",
            amount, lockup_days, sender_id
        ));
        
        // Ensure that only SIN tokens are accepted for staking
        assert_eq!(
            env::predecessor_account_id(),
            self.sin_token,
            "Only SIN tokens are accepted for staking"
        );
        
        // Ensure the staked amount is greater than zero
        assert!(amount > 0, "Stake amount must be greater than zero");
    
        // Use the sender_id directly since it represents the token sender
        let staker_id = sender_id;
    
        // Current timestamp for the staking record
        let start_timestamp = env::block_timestamp();
    
        // Fetch the staker's existing information or create a new record
        let mut staker_info = self.stakers.get(&staker_id).unwrap_or_else(|| StakerInfo {
            stakes: Vector::new(format!("stakes_{}", staker_id).as_bytes().to_vec()),
            total_rewards_claimed: 0,
        });
    
        // Create a new staking record
        let staking_record = StakingRecord {
            staked_tokens: amount, // Use the amount directly, as it's already a u128
            start_timestamp,
            lockup_period: lockup_days * DAY,
            claimed_rewards: 0,
        };
    
        // Add the new staking record to the staker's list
        staker_info.stakes.push(&staking_record);
    
        // Update the staker's information in the contract's state
        self.stakers.insert(&staker_id, &staker_info);
    }

    pub fn distribute_rewards(&mut self, amount: U128) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner,
            "Only the owner can distribute rewards"
        );

        // Check if the available funds are sufficient
        assert!(
            amount.0 <= self.reward_distribution.total_reward_pool,
            "Insufficient funds in the reward pool for distribution"
        );

        let reward_pool = amount.0;
        let mut total_tpes = 0.0;
        let mut staker_tpes: HashMap<AccountId, Vec<(usize, f64)>> = HashMap::new();

        for (staker_id, mut staker_info) in self.stakers.iter() {
            let mut stakes_tpes = vec![];

            for i in 0..staker_info.stakes.len() {
                let stake = staker_info.stakes.get(i as u64).unwrap();
                let days_staked = (env::block_timestamp() - stake.start_timestamp) / DAY;

                if days_staked >= 30 {
                    let weight = self.get_staking_weight(days_staked * DAY);
                    let tpes = weight * stake.staked_tokens as f64;

                    stakes_tpes.push((i as usize, tpes));
                    total_tpes += tpes;
                }

                staker_info.stakes.replace(i as u64, &stake);
            }

            staker_tpes.insert(staker_id.clone(), stakes_tpes);
        }

        for (staker_id, stakes_tpes) in staker_tpes {
            let mut staker_info = self.stakers.get(&staker_id).unwrap();

            for (i, tpes) in stakes_tpes {
                let reward_percentage = reward_pool as f64 / total_tpes;
                let reward = (tpes * reward_percentage) as Balance;

                let mut stake = staker_info.stakes.get(i as u64).unwrap();
                stake.claimed_rewards += reward;
                staker_info.stakes.replace(i as u64, &stake);
            }

            self.stakers.insert(&staker_id, &staker_info);
        }

        // Deduct distributed amount from the total reward pool
        self.reward_distribution.total_reward_pool -= reward_pool;
        self.reward_distribution.last_distributed = env::block_timestamp();

        env::log_str(&format!(
            "Distributed {} SIN tokens to stakers",
            reward_pool
        ));
    }

    #[payable]
    pub fn claim_reward(&mut self, stake_index: u64) {
        let staker_id = env::predecessor_account_id();
        let mut staker_info = self.stakers.get(&staker_id).expect("Staker not found");
    
        // Ensure the stake index is valid
        assert!(
            stake_index < staker_info.stakes.len(),
            "Invalid staking record index"
        );
    
        // Fetch the specified staking record
        let mut stake = staker_info.stakes.get(stake_index).expect("Stake not found");
        let rewards_to_claim = stake.claimed_rewards;
    
        // Ensure there are rewards to claim
        assert!(rewards_to_claim > 0, "No rewards available to claim for this stake");
    
        // Reset claimed rewards for the stake
        stake.claimed_rewards = 0;
        staker_info.stakes.replace(stake_index, &stake);
    
        // Update total rewards claimed
        staker_info.total_rewards_claimed += rewards_to_claim;
        self.stakers.insert(&staker_id, &staker_info);
    
        // Transfer the rewards to the staker's account
        Promise::new(self.sin_token.clone()).function_call(
            "ft_transfer".to_string(),                          // Method name
            serde_json::to_vec(&json!({                         // Arguments
                "receiver_id": staker_id,
                "amount": U128(rewards_to_claim),
            }))
            .expect("Failed to serialize ft_transfer arguments"), 
            NearToken::from_yoctonear(1),                                                  // Attach 1 yoctoNEAR
            Gas::from_tgas(50),                                 // Attach 50 TGas
        );
    
        env::log_str(&format!(
            "Transferred {} SIN tokens to {} for staking record {}",
            rewards_to_claim, staker_id, stake_index
        ));
    }

    #[payable]
    pub fn unstake_tokens(&mut self, stake_index: u64) {
        let staker_id = env::predecessor_account_id();
        let mut staker_info = self.stakers.get(&staker_id).expect("Staker not found");

        // Ensure the stake index is valid
        assert!(
            stake_index < staker_info.stakes.len(),
            "Invalid staking record index"
        );

        // Fetch the specific staking record
        let stake = staker_info.stakes.get(stake_index).expect("Stake not found");

        // Check if the lockup period has elapsed
        let current_time = env::block_timestamp();
        assert!(
            current_time >= stake.start_timestamp + stake.lockup_period,
            "Cannot unstake before the lockup period ends"
        );

        // Get the staked tokens to be unstaked
        let staked_tokens = stake.staked_tokens;

        // Remove the staking record from the staker's stakes
        staker_info.stakes.swap_remove(stake_index);

        // Update the staker's info
        self.stakers.insert(&staker_id, &staker_info);

        // Transfer the staked tokens back to the staker
        Promise::new(self.sin_token.clone()).function_call(
            "ft_transfer".to_string(),                          // Method name
            serde_json::to_vec(&json!({                         // Arguments
                "receiver_id": staker_id,
                "amount": U128(staked_tokens),
            }))
            .expect("Failed to serialize ft_transfer arguments"), 
            NearToken::from_yoctonear(1),                                                  // Attach 1 yoctoNEAR
            Gas::from_tgas(50),                                 // Attach 50 TGas
        );

        env::log_str(&format!(
            "Unstaked {} SIN tokens for {} from staking record {}",
            staked_tokens, staker_id, stake_index
        ));
    }

    pub fn get_funding_records(&self) -> Vec<FundingRecord> {
        self.reward_distribution
            .funding_records
            .iter()
            .collect::<Vec<FundingRecord>>()
    }

    // Helper functions
    pub fn get_staking_weight(&self, days_staked: u64) -> f64 {
        for (&threshold, &weight) in &self.staking_weight {
            if days_staked <= threshold {
                return weight;
            }
        }
        1.0
    }

    pub fn get_staking_info(&self, staker_id: AccountId) -> Vec<StakingRecord> {
        let staker_info = self.stakers.get(&staker_id).expect("Staker not found");
        staker_info.stakes.iter().collect()
    }

    pub fn get_next_reward_distribution(&self) -> u64 {
        let now = env::block_timestamp();
        let next_distribution = self.reward_distribution.last_distributed + MONTH;
        if next_distribution > now {
            (next_distribution - now) / DAY
        } else {
            0
        }
    }

    pub fn get_last_reward_distribution(&self) -> u64 {
        self.reward_distribution.last_distributed
    }

    pub fn get_available_reward(&self) -> u128 {
        self.reward_distribution.total_reward_pool
    }

    pub fn calculate_current_apr(&self) -> f64 {
        let total_reward_pool = self.reward_distribution.total_reward_pool;
        let total_staked_tokens: u128 = self
            .stakers
            .iter()
            .map(|(_, staker_info)| {
                staker_info
                    .stakes
                    .iter()
                    .map(|stake| stake.staked_tokens)
                    .sum::<u128>()
            })
            .sum();

        if total_reward_pool == 0 || total_staked_tokens == 0 {
            return 0.0;
        }

        // Calculate Estimated APR
        let estimated_apr = (total_reward_pool as f64 / total_staked_tokens as f64) * 100.0;
        estimated_apr
    }

    pub fn get_funding_details(&self) -> Vec<FundingRecord> {
        self.reward_distribution
            .funding_records
            .iter()
            .collect::<Vec<FundingRecord>>()
    }

    pub fn get_user_rewards(&self, staker_id: AccountId) -> serde_json::Value {
        if let Some(staker_info) = self.stakers.get(&staker_id) {
            let total_claimed = staker_info.total_rewards_claimed;
            let mut total_unclaimed: u128 = 0;
            let mut total_staked_tokens: u128 = 0;
    
            let stake_details: Vec<serde_json::Value> = staker_info
                .stakes
                .iter()
                .map(|stake| {
                    total_unclaimed = total_unclaimed
                        .checked_add(stake.claimed_rewards)
                        .unwrap_or_else(|| {
                            env::panic_str("Overflow in total_unclaimed calculation");
                        });
    
                    total_staked_tokens = total_staked_tokens
                        .checked_add(stake.staked_tokens)
                        .unwrap_or_else(|| {
                            env::panic_str("Overflow in total_staked_tokens calculation");
                        });
    
                    json!({
                        "staked_tokens": stake.staked_tokens.to_string(), // Serialize as string for safety
                        "start_timestamp": stake.start_timestamp,
                        "lockup_period": stake.lockup_period,
                        "claimed_rewards": stake.claimed_rewards.to_string() // Serialize as string for safety
                    })
                })
                .collect();
    
            json!({
                "total_staked_tokens": total_staked_tokens.to_string(),
                "total_claimed_rewards": total_claimed.to_string(),
                "total_unclaimed_rewards": total_unclaimed.to_string(),
                "stake_details": stake_details
            })
        } else {
            json!({
                "error": "No staking information found for this user"
            })
        }
    }
}