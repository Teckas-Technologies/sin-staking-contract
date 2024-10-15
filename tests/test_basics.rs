use near_sdk::test_utils::{accounts, VMContextBuilder}; 
use near_sdk::{testing_env, AccountId, NearToken};  
pub type Balance = u128;
use sin_staking_contract::{StakingContract, NFTTier};
use near_sdk::json_types::U128;

// Setup context for testing
fn get_context(predecessor: AccountId, deposit: Balance, timestamp: u64) -> VMContextBuilder {
    let mut builder = VMContextBuilder::new();
    builder.predecessor_account_id(predecessor).attached_deposit(NearToken::from_yoctonear(deposit)).block_timestamp(timestamp);
    builder
}

#[test]
fn test_new_contract() {
    let context = get_context(accounts(0), 0, 0);
    testing_env!(context.build());

    let contract = StakingContract::new(U128(1_000_000_000), U128(10));

    assert_eq!(contract.reward_pool, 1_000_000_000);
    assert_eq!(contract.total_staked, 0);
    assert_eq!(contract.reward_rate, 10);
}

#[test]
fn test_stake_tokens() {
    let context = get_context(accounts(1), 1000, 1_000_000_000_000);  // Some deposit and timestamp
    testing_env!(context.build());

    let mut contract = StakingContract::new(U128(1_000_000_000), U128(10));

    contract.stake_tokens(accounts(1), U128(500));

    let staker = contract.stakers.get(&accounts(1)).unwrap();
    assert_eq!(staker.staked_amount, 500);
    assert_eq!(contract.total_staked, 500);
}

#[test]
fn test_stake_nft() {
    let context = get_context(accounts(1), 1000, 1_000_000_000_000);
    testing_env!(context.build());

    let mut contract = StakingContract::new(U128(1_000_000_000), U128(10));

    contract.stake_nft(accounts(1), "Queen".to_string());

    let staker = contract.stakers.get(&accounts(1)).unwrap();
    assert_eq!(staker.nft_tier.unwrap(), NFTTier::Queen);

    let nft_tier = contract.nft_tiers.get(&accounts(1)).unwrap();
    assert_eq!(nft_tier, NFTTier::Queen);
}

#[test]
fn test_calculate_rewards() {
    let context = get_context(accounts(1), 1000, 1_000_000_000_000);  // Some deposit and timestamp
    testing_env!(context.build());

    let mut contract = StakingContract::new(U128(1_000_000_000), U128(10));

    // Stake some tokens and simulate passage of time
    contract.stake_tokens(accounts(1), U128(500));

    let new_context = get_context(accounts(1), 1000, 1_000_000_000_000 + 10 * 1_000_000_000);  // Advance 10 seconds
    testing_env!(new_context.build());

    let rewards = contract.calculate_rewards(accounts(1));

    assert!(rewards.0 > 0);  // Check if rewards are calculated correctly
}

#[test]
fn test_claim_rewards() {
    let context = get_context(accounts(1), 1000, 1_000_000_000_000);
    testing_env!(context.build());

    let mut contract = StakingContract::new(U128(1_000_000_000), U128(10));

    contract.stake_tokens(accounts(1), U128(500));

    // Simulate time passage of more than 30 days
    let new_context = get_context(accounts(1), 1000, 1_000_000_000_000 + 31 * 24 * 60 * 60 * 1_000_000_000);
    testing_env!(new_context.build());

    contract.claim_rewards(accounts(1));

    let staker = contract.stakers.get(&accounts(1)).unwrap();
    assert!(staker.rewards_claimed);  // Ensure rewards were claimed
}

#[test]
#[should_panic(expected = "Lock-up period has not passed.")]
fn test_claim_rewards_before_lockup() {
    let context = get_context(accounts(1), 1000, 1_000_000_000_000);
    testing_env!(context.build());

    let mut contract = StakingContract::new(U128(1_000_000_000), U128(10));

    contract.stake_tokens(accounts(1), U128(500));

    // Attempt to claim rewards before the lock-up period has passed
    contract.claim_rewards(accounts(1));
}

#[test]
fn test_unstake_tokens_and_nft() {
    let context = get_context(accounts(1), 1000, 1_000_000_000_000);
    testing_env!(context.build());

    let mut contract = StakingContract::new(U128(1_000_000_000), U128(10));

    // Stake tokens and NFTs
    contract.stake_tokens(accounts(1), U128(500));
    contract.stake_nft(accounts(1), "Worker".to_string());

    // Simulate time passage of more than 30 days
    let new_context = get_context(accounts(1), 1000, 1_000_000_000_000 + 31 * 24 * 60 * 60 * 1_000_000_000);
    testing_env!(new_context.build());

    contract.unstake(accounts(1));

    // Check that the staker is removed
    assert!(contract.stakers.get(&accounts(1)).is_none());
    assert!(contract.nft_tiers.get(&accounts(1)).is_none());
}

#[test]
#[should_panic(expected = "Lock-up period has not passed.")]
fn test_unstake_before_lockup() {
    let context = get_context(accounts(1), 1000, 1_000_000_000_000);
    testing_env!(context.build());

    let mut contract = StakingContract::new(U128(1_000_000_000), U128(10));

    contract.stake_tokens(accounts(1), U128(500));

    // Attempt to unstake before the lock-up period has passed
    contract.unstake(accounts(1));
}
