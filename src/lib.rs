use near_sdk::{env, near_bindgen, AccountId, Promise, PanicOnDefault};
use near_sdk::collections::LookupMap;
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum NFTTier {
    Queen,
    Worker,
    Drone,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Staker {
    pub staked_amount: u128,
    pub nft_tier: Option<NFTTier>,
    pub staked_at: u64,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct StakingContract {
    pub stakers: LookupMap<AccountId, Staker>,
    pub nft_tier_info: LookupMap<AccountId, NFTTier>,
    pub total_staked_tokens: u128,
    pub reward_pool: u128,
}

#[near_bindgen]
impl StakingContract {
    #[init]
    pub fn new(reward_pool: U128) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            stakers: LookupMap::new(b"s".to_vec()),
            nft_tier_info: LookupMap::new(b"n".to_vec()),
            total_staked_tokens: 0,
            reward_pool: reward_pool.0,
        }
    }

    pub fn stake_tokens(&mut self, amount: U128) {
        let account_id = env::predecessor_account_id();
        let staked_at = env::block_timestamp();

        assert!(amount.0 > 0, "Amount must be greater than 0");
        let mut staker = self.stakers.get(&account_id).unwrap_or(Staker {
            staked_amount: 0,
            nft_tier: None,
            staked_at: 0,
        });

        staker.staked_amount += amount.0;
        staker.staked_at = staked_at;

        self.stakers.insert(&account_id, &staker);

        self.total_staked_tokens += amount.0;

        env::log_str(&format!(
            "Staked {} tokens for account: {}",
            amount.0, account_id
        ));
    }

    pub fn stake_nft(&mut self, nft_tier: NFTTier) {
        let account_id = env::predecessor_account_id();
        let staked_at = env::block_timestamp();
        let mut staker = self.stakers.get(&account_id).unwrap_or(Staker {
            staked_amount: 0,
            nft_tier: None,
            staked_at: 0,
        });

        staker.nft_tier = Some(nft_tier.clone());
        staker.staked_at = staked_at;

        self.stakers.insert(&account_id, &staker);


        self.nft_tier_info.insert(&account_id, &nft_tier);


        env::log_str(&format!(
            "Staked an NFT with tier {:?} for account: {}",
            nft_tier, account_id
        ));
    }

    pub fn get_staker_info(&self, account_id: AccountId) -> Option<Staker> {
        self.stakers.get(&account_id)
    }

    pub fn get_total_staked_tokens(&self) -> U128 {
        U128(self.total_staked_tokens)
    }

    pub fn get_reward_pool(&self) -> U128 {
        U128(self.reward_pool)
    }
}
