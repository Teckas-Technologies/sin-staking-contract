use near_sdk::json_types::U128;
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault, PromiseOrValue, BorshStorageKey, NearToken};
use near_sdk::collections::LookupMap;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{Promise, Gas};
use near_sdk::ext_contract;

pub type Balance = u128;

pub type TokenId = String;
const LOCKUP_PERIOD: u64 = 30 * 24 * 60 * 60 * 1_000_000_000; // 30 days in nanoseconds
const GAS_FOR_NFT_METADATA: Gas = Gas::from_tgas(10);  // Gas for cross-contract call

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
    pub reward_pool: Balance, // SIN token balance pool
    pub total_staked: Balance,
    pub reward_rate: Balance,
    pub nft_contract_id: AccountId,  // External NFT contract to check metadata
    pub sin_token_contract: AccountId,  // SIN token contract for transfers
}

#[near_bindgen]
impl StakingContract {
    #[init]
    pub fn new(reward_rate: U128, nft_contract_id: AccountId, sin_token_contract: AccountId) -> Self {
        Self {
            stakers: LookupMap::new(StorageKeys::Stakers),
            reward_pool: 0,
            total_staked: 0,
            reward_rate: reward_rate.0,
            nft_contract_id,
            sin_token_contract,
        }
    }

    // Handle the transfer of SIN tokens to fund the reward pool
    #[payable]
    pub fn fund_reward_pool(&mut self, amount: U128) {
        assert!(
            env::predecessor_account_id() == env::current_account_id(),
            "Only owner can fund the pool."
        );
        self.reward_pool += amount.0;
        env::log_str("Reward pool funded with SIN tokens.");
    }

    // Implement the ft_on_transfer to handle SIN tokens transferred to the contract
    #[allow(unused_variables)]
    #[payable]
    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        assert!(
            env::predecessor_account_id() == self.sin_token_contract,
            "Only SIN tokens are accepted."
        );

        // SIN tokens have been transferred, update the reward pool
        self.reward_pool += amount.0;

        PromiseOrValue::Value(U128(0)) // Returning 0 means we accepted the transfer
    }

    // Stake NFT based on external metadata
    pub fn stake_nft(&mut self, account_id: AccountId, nft_id: TokenId) -> Promise {
        let nft_contract = self.nft_contract_id.clone();
        self.get_nft_metadata(nft_contract, nft_id.clone()).then(
            Self::ext(env::current_account_id()).on_metadata_response(account_id, nft_id)
        )
    }

    #[private]
    pub fn on_metadata_response(&mut self, account_id: AccountId, nft_id: TokenId, #[callback_result] call_result: Result<String, near_sdk::PromiseError>) {
        let metadata = call_result.expect("Failed to get NFT metadata");

        let nft_tier = match metadata.as_str() {
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

        staker.nft_tier = Some(nft_tier.clone());
        staker.staked_at = env::block_timestamp();
        staker.rewards_claimed = false;

        self.stakers.insert(&account_id, &staker);

        env::log_str("NFT staked successfully.");
    }

    // Calculate and claim SIN token rewards
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

        Promise::new(env::predecessor_account_id()).function_call(
            "ft_transfer".to_string(),
            serde_json::json!({
                "receiver_id": account_id,
                "amount": rewards,
            })
            .to_string()
            .into_bytes(),
            NearToken::from_yoctonear(1),  // This is the attached deposit (you may want to change this if you don't need a deposit)
            Gas::from_tgas(5),  // Specify the gas to use for the function call
        );

        // Update contract state
        self.reward_pool -= rewards;
        staker.rewards_claimed = true;
        self.stakers.insert(&account_id, &staker);

        env::log_str("Rewards claimed successfully.");
    }

    // Helper function to call the NFT contract and retrieve NFT metadata
    pub fn get_nft_metadata(&self, nft_contract_id: AccountId, token_id: TokenId) -> Promise {
        Promise::new(nft_contract_id).function_call(
            "nft_metadata".to_string(),
            serde_json::json!({
                "token_id": token_id
            }).to_string().into_bytes(),
            NearToken::from_yoctonear(0),  // Attached deposit
            Gas::from_tgas(5),  // Specifying gas
        )
    }

    // Utility function to calculate rewards based on the staked amount and reward rate
    pub fn calculate_rewards(&self, account_id: AccountId) -> U128 {
        let staker = self.stakers.get(&account_id).expect("No staking found for user.");
        let staking_duration = env::block_timestamp() - staker.staked_at;

        let rewards = (staking_duration as u128 * self.reward_rate) / 1_000_000_000;

        U128(rewards)
    }

    // Utility function to get contract reward pool balance
    pub fn get_reward_pool(&self) -> U128 {
        U128(self.reward_pool)
    }
}

// External contract interface for cross-contract calls
#[ext_contract(ext)]
pub trait ExtContract {
    fn nft_metadata(&self, token_id: TokenId) -> String;
}
