use near_sdk::collections::{LookupMap};
use near_sdk::{env, near_bindgen, AccountId, BorshStorageKey, NearToken, PanicOnDefault, Promise};
use near_sdk::json_types::U128;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
pub type Balance = u128;

// Constants
const LOCKUP_PERIOD: u64 = 30 * 24 * 60 * 60 * 1_000_000_000; // 30 days in nanoseconds

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    Stakers,
    NftTiers,
}

#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize)]
pub enum NFTTier {
    Queen,
    Worker,
    Drone,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct Staker {
    pub staked_amount: Balance,
    pub nft_tier: Option<NFTTier>,
    pub staked_at: u64,
    pub rewards_claimed: bool,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct StakingContract {
    pub stakers: LookupMap<AccountId, Staker>,
    pub reward_pool: Balance,
    pub total_staked: Balance,
    pub reward_rate: Balance,
    pub nft_tiers: LookupMap<AccountId, NFTTier>,
}

#[near_bindgen]
impl StakingContract {
    #[init]
    pub fn new(reward_pool: U128, reward_rate: U128) -> Self {
        Self {
            stakers: LookupMap::new(StorageKeys::Stakers),
            reward_pool: reward_pool.0,
            total_staked: 0,
            reward_rate: reward_rate.0,
            nft_tiers: LookupMap::new(StorageKeys::NftTiers),
        }
    }

    pub fn stake_tokens(&mut self, account_id: AccountId, amount: U128) {
        let mut staker = self.stakers.get(&account_id).unwrap_or_else(|| Staker {
            staked_amount: 0,
            nft_tier: None,
            staked_at: env::block_timestamp(),
            rewards_claimed: false,
        });

        staker.staked_amount += amount.0;
        staker.staked_at = env::block_timestamp();
        staker.rewards_claimed = false;

        self.total_staked += amount.0;

        self.stakers.insert(&account_id, &staker);

        env::log_str("Tokens staked successfully.");
    }

    pub fn stake_nft(&mut self, account_id: AccountId, nft_tier: String) {
        let tier = match nft_tier.as_str() {
            "Queen" => NFTTier::Queen,
            "Worker" => NFTTier::Worker,
            "Drone" => NFTTier::Drone,
            _ => panic!("Invalid NFT tier"),
        };

        let mut staker = self.stakers.get(&account_id).unwrap_or_else(|| Staker {
            staked_amount: 0,
            nft_tier: None,
            staked_at: env::block_timestamp(),
            rewards_claimed: false,
        });

        staker.nft_tier = Some(tier.clone());
        staker.staked_at = env::block_timestamp();
        staker.rewards_claimed = false;

        self.stakers.insert(&account_id, &staker);
        self.nft_tiers.insert(&account_id, &tier);

        env::log_str("NFT staked successfully.");
    }

    pub fn calculate_rewards(&self, account_id: AccountId) -> U128 {
        if let Some(staker) = self.stakers.get(&account_id) {
            let current_time = env::block_timestamp();
            let duration_staked = current_time.saturating_sub(staker.staked_at); // Duration in nanoseconds
            
            if duration_staked < LOCKUP_PERIOD {
                // If the lockup period hasn't passed, return 0 rewards.
                return U128(0);
            }
    
            // Convert nanoseconds to seconds (1 second = 1_000_000_000 nanoseconds)
            let duration_in_seconds = duration_staked / 1_000_000_000;
    
            // Calculate the base reward as: staked_amount * reward_rate * duration_in_seconds
            let base_reward = staker.staked_amount
                .checked_mul(self.reward_rate)
                .expect("Multiplication overflow")
                .checked_mul(duration_in_seconds as u128)
                .expect("Multiplication overflow");
    
            // Final reward calculation - Divide by a large number to normalize (or scale based on tokenomics)
            // Adjust this scaling factor depending on the magnitude of your reward_rate and the expected token supply.
            let reward = base_reward / 1_000_000_000; // Scale down
    
            // Return the reward as U128
            U128(reward)
        } else {
            U128(0)
        }
    }
    

    // Claim Rewards
    pub fn claim_rewards(&mut self, account_id: AccountId) {
        let mut staker = self.stakers.get(&account_id).expect("No staking found for user.");

        assert!(
            env::block_timestamp() >= staker.staked_at + LOCKUP_PERIOD,
            "Lock-up period has not passed."
        );
        assert!(
            !staker.rewards_claimed,
            "Rewards already claimed."
        );

        let rewards = self.calculate_rewards(account_id.clone()).0;

        assert!(
            rewards <= self.reward_pool,
            "Not enough tokens in reward pool."
        );

        // Transfer rewards
        Promise::new(account_id.clone()).transfer(NearToken::from_yoctonear(rewards));

        // Update contract state
        self.reward_pool -= rewards;
        staker.rewards_claimed = true;
        self.stakers.insert(&account_id, &staker);

        env::log_str("Rewards claimed successfully.");
    }

    // Unstake Tokens or NFTs
    pub fn unstake(&mut self, account_id: AccountId) {
        let staker = self.stakers.get(&account_id).expect("No staking found for user.");

        assert!(
            env::block_timestamp() >= staker.staked_at + LOCKUP_PERIOD,
            "Lock-up period has not passed."
        );

        // Transfer staked tokens back to the user
        if staker.staked_amount > 0 {
            Promise::new(account_id.clone()).transfer( NearToken::from_yoctonear(staker.staked_amount));
        }

        // Remove NFT tier if any
        if staker.nft_tier.is_some() {
            self.nft_tiers.remove(&account_id);
        }

        // Update contract state
        self.total_staked -= staker.staked_amount;
        self.stakers.remove(&account_id);

        env::log_str("Tokens and/or NFTs unstaked successfully.");
    }

    // Utility function to create test tokens for staking
    pub fn create_test_tokens(&mut self, account_id: AccountId, amount: U128) {
        // This is a simple mock to simulate token creation for test purposes.
        Promise::new(account_id.clone()).transfer(NearToken::from_yoctonear(amount.0));
        env::log_str("Test tokens created.");
    }

    // Utility function to mint NFTs with metadata defining tiers
    pub fn mint_test_nft(&mut self, account_id: AccountId, tier: String) {
        let nft_tier = match tier.as_str() {
            "Queen" => NFTTier::Queen,
            "Worker" => NFTTier::Worker,
            "Drone" => NFTTier::Drone,
            _ => panic!("Invalid tier"),
        };
        self.nft_tiers.insert(&account_id, &nft_tier);
        env::log_str(&format!("Minted NFT of tier: {}", tier));
    }

    // Get staker details by account ID
    pub fn get_staker(&self, account_id: AccountId) -> Option<Staker> {
        self.stakers.get(&account_id)
    }

    // Get staked amount by account ID
    pub fn get_staked_amount(&self, account_id: AccountId) -> U128 {
        if let Some(staker) = self.stakers.get(&account_id) {
            U128(staker.staked_amount)
        } else {
            U128(0)
        }
    }

    // Get NFT tier staked by account ID
    pub fn get_nft_tier(&self, account_id: AccountId) -> Option<String> {
        if let Some(tier) = self.nft_tiers.get(&account_id) {
            Some(match tier {
                NFTTier::Queen => "Queen".to_string(),
                NFTTier::Worker => "Worker".to_string(),
                NFTTier::Drone => "Drone".to_string(),
            })
        } else {
            None
        }
    }

    // Get the total staked amount in the contract
    pub fn get_total_staked(&self) -> U128 {
        U128(self.total_staked)
    }

    // Get the reward pool balance
    pub fn get_reward_pool(&self) -> U128 {
        U128(self.reward_pool)
    }

    // Get the current reward rate
    pub fn get_reward_rate(&self) -> U128 {
        U128(self.reward_rate)
    }

    // Check if the staker has claimed rewards
    pub fn has_claimed_rewards(&self, account_id: AccountId) -> bool {
        if let Some(staker) = self.stakers.get(&account_id) {
            staker.rewards_claimed
        } else {
            false
        }
    }

    // Get lockup period in seconds (for reference)
    pub fn get_lockup_period(&self) -> u64 {
        LOCKUP_PERIOD / 1_000_000_000 // Convert from nanoseconds to seconds
    }
}
