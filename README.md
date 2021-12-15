## Requirements

	1. Stake NFT for gorillar tokenomics.

	2. NFT holders will get airdrop 10 $GLUE tokens everyday for per staked NFT.

	3. Staked NFT will be unlock and return to original holder after 7 days from just staked.

	4. $GLUE tokens should be list on Raidyum or Synthetify to have monetary value.

	5. Stakers can claim $GULE tokens reward which are accumulated while staking.

	6. When new collection released, NFT holders will use $GLUE to gain new NFT.

## Algorithm

1. Init Pool

First we need to create pool account. I implemented a pool creator can set parameters like period.

```Pool
	owner(pubkey) : Owner of this pool account.

	rand(pubkey) : Pool have to be PDA. We also have to generate pool infinitely. So I introduced random pubkey as one solution.

	reward_mint(pubkey) : In our case, $GLUE token mint.

	reward_account(pubkey) : This is $GLUE token account. This account's owner must be pool address.

	reward_amount(u64) : Holders will get airdrop $GLUE (amount of this value) every period for per staked NFT. In our case, 10

	period(i64) : time interval(unit is second) holders can airdrop. In our case, 24*60*60=86400(s)

	withdrawable(u8) : withdrawable times. In our case, 7(days)

	stake_collection(string) : nft collection symbol we can stake. In our case, gorilla nft's symbol

	bump(u8) : PDA's bump
```

2. Stake

We have to check accounts if nft is correct, pool is matched and so on.

And then you transfer to pool.

All staking data is stored in StakeData account.

```StakeData
	unstaked(bool) : If true, you already redeem staked nft.
	
	owner(pubkey) : staker account

	pool(pubkey) : pool account

	account(pubkey) : When you stake nft, you transfer nft to pool. This account is nft account of pool.

	stake_time(i64) : when you stake nft

	withdrawn_number : period number that you already withdrawn
```

3. Unstake

First we check accounts like "Staking". You are not allowed to unstake before staking time reachs limit(7 days in our case)

4. Claim

You can get withdrawable amount of one nft with this formula.

```withdrawable amount
	cur_number = (cur_time - stake_data.stake_time) / pool.peirod

	amount = pool.reward_amount * (cur_number - stake_data.withdrawn_number)
```

Finally, we change withdraw_number of stake_data.

## Airdrop vs Mint

We can implement both holders mint reward and pool sends token to holders.

In the case of "Mint", we don't need reward_account of pool anymore. And Pool must be mint authority of "reward token".

There are only a few differences between "Airdrop" and "Mint"
