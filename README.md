# sin-staking-contract

cargo-near-new-project-description

## How to Build Locally?

Install [`cargo-near`](https://github.com/near/cargo-near) and run:

```bash
cargo near build
```

## How to Test Locally?

```bash
cargo test
```

## How to Deploy?

Deployment is automated with GitHub Actions CI/CD pipeline.
To deploy manually, install [`cargo-near`](https://github.com/near/cargo-near) and run:

```bash
cargo near deploy <account-id>
```

## Initialise the contract
``
near call your-account.testnet new '{"reward_pool": "1000000000000000000000000"}' --accountId your-account.testnet
``

## Stake tokens

``
near call your-account.testnet stake_tokens '{"amount": "100000000000000000000"}' --accountId your-account.testnet --depositYocto 1
``

## Stake NFT
``
near call your-account.testnet stake_nft '{"nft_tier": "Queen"}' --accountId your-account.testnet --depositYocto 1
``

## View Staker Info
``
near view your-account.testnet get_staker_info '{"account_id": "your-account.testnet"}'
``

## View Staked Tokens
``
near view your-account.testnet get_total_staked_tokens
``

## Useful Links

- [cargo-near](https://github.com/near/cargo-near) - NEAR smart contract development toolkit for Rust
- [near CLI](https://near.cli.rs) - Interact with NEAR blockchain from command line
- [NEAR Rust SDK Documentation](https://docs.near.org/sdk/rust/introduction)
- [NEAR Documentation](https://docs.near.org)
- [NEAR StackOverflow](https://stackoverflow.com/questions/tagged/nearprotocol)
- [NEAR Discord](https://near.chat)
- [NEAR Telegram Developers Community Group](https://t.me/neardev)
- NEAR DevHub: [Telegram](https://t.me/neardevhub), [Twitter](https://twitter.com/neardevhub)
