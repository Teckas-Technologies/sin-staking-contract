# SIN Token Staking contract

## To create necessary accounts

```
near create-account sin-contract-account.testnet --masterAccount varathatest.testnet

near create-account sin-owner-account.testnet --masterAccount varathatest.testnet

near create-account sin-funding-wallet.testnet --masterAccount varathatest.testnet

near create-account sin-staker-account.testnet --masterAccount varathatest.testnet
```

## To Build the contract
```
cargo build --target wasm32-unknown-unknown --release
```

## To deploy Contract
```
near deploy sin-contract-account.testnet target/wasm32-unknown-unknown/release/sin_staking_contract.wasm
```

## To initialise the contract
```
near call sin-contract-account.testnet new '{"owner": "sin-owner-account.testnet", "funding_wallet": "sin-funding-wallet.testnet"}' --accountId sin-owner-account.testnet
```

## To fund the reward Pool
```
near call sin-contract-account.testnet fund_reward_pool '{}' --accountId sin-owner-account.testnet --depositYocto 1000000000000000000000000
```

## To Stake Sin Tokens
```
near call sin-contract-account.testnet stake '{"amount": "500000000000000000000000"}' --accountId sin-staker-account.testnet --depositYocto 500000000000000000000000
```

## To Check staking info
```
near view sin-contract-account.testnet get_staking_info '{"account_id": "sin-staker-account.testnet"}'
```

## To Check for next calculation
```
near view sin-contract-account.testnet get_next_reward_calculation_day '{}'
```

## To Claim Rewards
```
near call sin-contract-account.testnet claim_rewards '{}' --accountId sin-staker-account.testnet
```

## Unstake tokens
```
near call sin-contract-account.testnet unstake '{}' --accountId sin-staker-account.testnet
```

## To View Contract state
```
near state sin-contract-account.testnet
```





# To create test token

## Clone the code
```
git clone https://github.com/near/near-sdk-rs.git 
```
## Navigate to fingible token directory
```
cd near-sdk-rs/examples/fungible-token 
```

## Login to Near wallet
```
near login
```

## Create token contract
```
near create-account sin-test-tkn.testnet --masterAccount varathatest.testnet --initialBalance 10
```

## Build the contract
```
./build.sh    
```

## Deploy the contract
```
near deploy sin-test-tkn.testnet res/fungible_token.wasm 
```
## Initialise the contract
```
near call sin-test-tkn.testnet new_default_meta '{"owner_id": "varathatest.testnet", "total_supply": "1000000000000000000000000", "name": "SIN Token", "symbol": "SIN"}' --accountId varathatest.testnet
```
## View total supply
```
near view sin-test-tkn.testnet ft_total_supply
```
## View balance
```
near view sin-test-tkn.testnet ft_balance_of '{"account_id": "varathatest.testnet"}'
```

## Initiate storage balance
```
near call sin-test-tkn.testnet storage_deposit '{"account_id": "sin-staker-account.testnet"}' --accountId varathatest.testnet --depositYocto 1250000000000000000000
```

## Check Stoage balance
```
near view sin-test-tkn.testnet storage_balance_of '{"account_id": "sin-staker-account.testnet"}'
```

## Transfer tokens
```
near call sin-test-tkn.testnet ft_transfer '{"receiver_id": "sin-staker-account.testnet", "amount": "500000000000000000000", "memo": "Reward distribution"}' --accountId varathatest.testnet --depositYocto 1
```
