use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{UnorderedMap, Vector},
    env, near_bindgen, AccountId, PanicOnDefault, NearToken
};
use near_sdk::{json_types::U128, Gas};
use serde_json::json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use near_contract_standards::fungible_token::Balance;
use near_sdk::Promise;

const DAY: u64 = 86400; // Seconds in a day
const MONTH: u64 = 30 * DAY; // Approximate seconds in a month
const MONTHLY_REWARD: Balance = 2_500_000_000; // Monthly reward pool

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

#[derive(BorshDeserialize, BorshSerialize)]
pub struct RewardDistribution {
    pub total_reward_pool: Balance,
    pub last_distributed: u64, // Timestamp of last reward distribution
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
    
        // Parse `msg` as JSON to extract lockup_days
        let lockup_days: u64 = if msg.is_empty() {
            30 // Default lockup period if no message provided
        } else {
            match serde_json::from_str::<serde_json::Value>(&msg) {
                Ok(parsed_msg) => parsed_msg["lockup_days"].as_u64().unwrap_or(30),
                Err(_) => panic!("Invalid message format in ft_on_transfer"),
            }
        };
    
        // Call the staking logic
        self.stake_tokens(sender_id, amount.0, lockup_days);
    
        // Return 0 to indicate all tokens were accepted
        U128(0)
    }


    // Owner funds reward pool (only SIN tokens allowed)
    #[payable]
    pub fn fund_reward_pool(&mut self, amount: U128) {
        assert_eq!(
            env::predecessor_account_id(),
            self.sin_token,
            "Only SIN tokens are accepted for funding"
        );
        assert!(amount.0 > 0, "Funding amount must be greater than zero");
        self.reward_distribution.total_reward_pool += amount.0;
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

    // Distribute rewards
    pub fn distribute_rewards(&mut self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner,
            "Only owner can distribute rewards"
        );

        let reward_pool = MONTHLY_REWARD;
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
            
                    // Convert `u64` to `usize` for compatibility with `staker_tpes`
                    stakes_tpes.push((i as usize, tpes));
                    total_tpes += tpes;
                }
            
                // Replace the updated stake
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

        self.reward_distribution.last_distributed = env::block_timestamp();
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
    // assert!(
    //     current_time >= stake.start_timestamp + stake.lockup_period,
    //     "Cannot unstake before the lockup period ends"
    // );

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
}