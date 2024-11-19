use near_sdk::{
    env, near_bindgen, AccountId, Promise, PanicOnDefault, PromiseOrValue, NearToken
};
use near_sdk::json_types::U128;
use near_sdk::collections::UnorderedMap;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use serde::{Serialize, Deserialize};

type Balance = u128; 
const ONE_MONTH_IN_SECONDS: u64 = 2_592_000;
const SIN_TOKEN_CONTRACT: &str = "sin.token.contract";
const REWARD_POOL_PER_MONTH: Balance = 2_500_000_000_000_000_000_000_000;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct TokenStakingContract {
    owner: AccountId,
    funding_wallet: AccountId,
    staking_info: UnorderedMap<AccountId, StakingInfo>,
    total_staked_points: f64,
    reward_pool: Balance,
    last_distribution_timestamp: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct StakingInfo {
    amount: Balance,
    start_time: u64,
    lockup_duration: u64,
    weight: f64,
    claimed: bool,
}

#[near_bindgen]
impl TokenStakingContract {
    #[init]
    pub fn new(owner: AccountId, funding_wallet: AccountId) -> Self {
        Self {
            owner,
            funding_wallet,
            staking_info: UnorderedMap::new(b"s"),
            total_staked_points: 0.0,
            reward_pool: REWARD_POOL_PER_MONTH,
            last_distribution_timestamp: env::block_timestamp() / 1_000_000_000,
        }
    }

    #[payable]
    pub fn fund_reward_pool(&mut self) {
        let account_id = env::predecessor_account_id();
        assert_eq!(account_id, self.owner, "Only the contract owner can fund the reward pool.");
        let attached_amount = env::attached_deposit();
        assert!(attached_amount > NearToken::from_yoctonear(0), "You need to attach some tokens to fund the reward pool.");
        self.reward_pool +=  NearToken::as_yoctonear(&attached_amount);
        env::log_str(&format!("{} funded the reward pool with {} SIN tokens", account_id, attached_amount));
    }

    #[payable]
    pub fn stake(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128> {
        assert_eq!(env::predecessor_account_id(), SIN_TOKEN_CONTRACT, "Only SIN tokens are accepted.");
        assert!(amount.0 > 0, "You need to stake a positive amount of SIN tokens.");

        let start_time = env::block_timestamp() / 1_000_000_000;
        let weight = self.calculate_weight(1);

        let mut staking_info = self.staking_info.get(&sender_id).unwrap_or_else(|| StakingInfo {
            amount: 0,
            start_time,
            lockup_duration: ONE_MONTH_IN_SECONDS,
            weight,
            claimed: false,
        });

        staking_info.amount += amount.0;
        staking_info.start_time = start_time;
        self.total_staked_points += amount.0 as f64 * weight;

        self.staking_info.insert(&sender_id, &staking_info);

        env::log_str(&format!("{} staked {} SIN tokens", sender_id, amount.0));
        PromiseOrValue::Value(U128(0)) // Indicates successful handling
    }

    pub fn claim_rewards(&mut self) {
        let account_id = env::predecessor_account_id();
        let mut staking_info = self.staking_info.get(&account_id).expect("No staking information found for this account.");

        let current_time = env::block_timestamp() / 1_000_000_000;
        assert!(
            current_time >= staking_info.start_time + staking_info.lockup_duration,
            "Rewards can only be claimed after the lock-up period."
        );

        let reward = self.calculate_rewards(account_id.clone());
        self.transfer_from_funding_wallet(account_id.clone(), reward);

        staking_info.claimed = true;
        self.staking_info.insert(&account_id, &staking_info);
        env::log_str("Rewards claimed successfully!");
    }

    pub fn unstake(&mut self) {
        let account_id = env::predecessor_account_id();
        let mut staking_info = self.staking_info.get(&account_id).expect("No staking information found for this account.");

        let elapsed_time = (env::block_timestamp() / 1_000_000_000) - staking_info.start_time;
        assert!(
            elapsed_time >= staking_info.lockup_duration,
            "Cannot unstake before completing the lock-up period."
        );

        self.transfer_from_funding_wallet(account_id.clone(), staking_info.amount);
        self.total_staked_points -= staking_info.amount as f64 * staking_info.weight;
        self.staking_info.remove(&account_id);
        env::log_str("Tokens unstaked successfully!");
    }

    fn calculate_weight(&self, months: u64) -> f64 {
        match months {
            1..=3 => 1.0,
            4..=6 => 1.5,
            7..=9 => 2.0,
            _ => 2.5,
        }
    }

    fn calculate_rewards(&self, account_id: AccountId) -> Balance {
        let staking_info = self.staking_info.get(&account_id).expect("No staking information found for this account.");
        let reward_percentage = self.reward_pool as f64 / self.total_staked_points;
        let tpes = staking_info.amount as f64 * staking_info.weight;
        (tpes * reward_percentage) as Balance
    }

    fn transfer_from_funding_wallet(&self, to: AccountId, amount: Balance) {
        Promise::new(to).transfer(NearToken::from_yoctonear(amount));
    }

    pub fn get_total_staked_points(&self) -> f64 {
        self.total_staked_points
    }

    pub fn get_reward_pool_balance(&self) -> Balance {
        self.reward_pool
    }

    pub fn get_user_staking_info(&self, account_id: AccountId) -> Option<StakingInfo> {
        self.staking_info.get(&account_id)
    }

    pub fn calculate_user_rewards(&self, account_id: AccountId) -> Balance {
        let staking_info = self.staking_info.get(&account_id).unwrap_or_else(|| {
            env::panic_str("No staking information found for this account.");
        });
    
        let current_time = env::block_timestamp() / 1_000_000_000;
        if current_time < staking_info.start_time + staking_info.lockup_duration {
            return 0; // Lockup period not complete, no rewards available
        }
    
        let reward_percentage = self.reward_pool as f64 / self.total_staked_points;
        let tpes = staking_info.amount as f64 * staking_info.weight;
        (tpes * reward_percentage) as Balance
    }
    
}