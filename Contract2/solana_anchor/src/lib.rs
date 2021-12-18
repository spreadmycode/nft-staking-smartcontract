pub mod utils;
use borsh::{BorshDeserialize,BorshSerialize};
use {
    crate::utils::*,
    anchor_lang::{
        prelude::*,
        AnchorDeserialize,
        AnchorSerialize,
        Key,
        solana_program::{
            program_pack::Pack,
            sysvar::{clock::Clock},
            msg
        }      
    },
    spl_token::state,
    metaplex_token_metadata::{
        state::{
            MAX_SYMBOL_LENGTH,
        }
    }
};
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod solana_anchor {
    use super::*;

    pub fn init_pool(
        ctx : Context<InitPool>,
        _bump : u8,
        _reward_amount : u64,
        _period : i64,
        _withdrawable : u8,
        _stake_collection : String,
        ) -> ProgramResult {
        msg!("Init Pool");
        let pool = &mut ctx.accounts.pool;
        let reward_account : state::Account = state::Account::unpack_from_slice(&ctx.accounts.reward_account.data.borrow())?;
        if reward_account.owner != pool.key() {
            return Err(PoolError::InvalidTokenAccount.into());
        }
        if reward_account.mint != *ctx.accounts.reward_mint.key {
            return Err(PoolError::InvalidTokenAccount.into());
        }
        if _period == 0 {
            return Err(PoolError::InvalidPeriod.into());
        }
        pool.owner = *ctx.accounts.owner.key;
        pool.rand = *ctx.accounts.rand.key;
        pool.reward_mint = *ctx.accounts.reward_mint.key;
        pool.reward_account = *ctx.accounts.reward_account.key;
        pool.reward_amount = _reward_amount;
        pool.period = _period;
        pool.withdrawable = _withdrawable;
        pool.stake_collection = _stake_collection;
        pool.bump = _bump;
        Ok(())
    }

    pub fn stake(
        ctx : Context<Stake>,
        ) -> ProgramResult {
        msg!("Stake");
        let pool = &ctx.accounts.pool;
        let clock = Clock::from_account_info(&ctx.accounts.clock)?;
        let source_nft_account : state::Account = state::Account::unpack_from_slice(&ctx.accounts.source_nft_account.data.borrow())?;
        let dest_nft_account : state::Account = state::Account::unpack_from_slice(&ctx.accounts.dest_nft_account.data.borrow())?;
        let nft_mint : state::Mint = state::Mint::unpack_from_slice(&ctx.accounts.nft_mint.data.borrow())?;
        let metadata : metaplex_token_metadata::state::Metadata =  metaplex_token_metadata::state::Metadata::from_account_info(&ctx.accounts.metadata)?;
        if nft_mint.decimals != 0 && nft_mint.supply != 1 {
            msg!("This mint is not proper nft");
            return Err(PoolError::InvalidTokenMint.into());
        }
        if metadata.mint != *ctx.accounts.nft_mint.key {
            msg!("Not match mint address");
            return Err(PoolError::InvalidMetadata.into());
        }
        if source_nft_account.owner == pool.key() {
            msg!("Source nft account's owner is not allowed to be Pool");
            return Err(PoolError::InvalidTokenAccount.into());
        }
        if source_nft_account.mint != *ctx.accounts.nft_mint.key {
            msg!("Not match mint address");
            return Err(PoolError::InvalidTokenAccount.into());
        }
        if dest_nft_account.owner != pool.key() {
            msg!("Destination nft account's owner must be Pool");
            return Err(PoolError::InvalidTokenAccount.into());
        }
        if (&metadata.data.symbol).eq(&pool.stake_collection) {
            msg!("Not match collection symbol");
            return Err(PoolError::InvalidMetadata.into());
        }

        spl_token_transfer_without_seed(
            TokenTransferParamsWithoutSeed{
                source : ctx.accounts.source_nft_account.clone(),
                destination : ctx.accounts.dest_nft_account.clone(),
                authority : ctx.accounts.owner.clone(),
                token_program : ctx.accounts.token_program.clone(),
                amount : 1,
            }
        )?;


        let stake_data = &mut ctx.accounts.stake_data;
        stake_data.owner = *ctx.accounts.owner.key;
        stake_data.pool = pool.key();
        stake_data.account = *ctx.accounts.dest_nft_account.key;
        stake_data.stake_time = clock.unix_timestamp;
        stake_data.withdrawn_number = 0;
        stake_data.unstaked = false;
        Ok(())
    }

    pub fn unstake(
        ctx : Context<Unstake>
        ) -> ProgramResult {
        msg!("Unstake");
        let pool = &ctx.accounts.pool;
        let stake_data = &mut ctx.accounts.stake_data;
        let clock = Clock::from_account_info(&ctx.accounts.clock)?;

        if stake_data.unstaked {
            return Err(PoolError::AlreadyUnstaked.into());
        }
        if clock.unix_timestamp < stake_data.stake_time + pool.period * pool.withdrawable as i64 {
            return Err(PoolError::InvalidTime.into());
        }
        if stake_data.owner != *ctx.accounts.owner.key {
            return Err(PoolError::InvalidStakeData.into());
        }
        if stake_data.pool != pool.key() {
            return Err(PoolError::InvalidStakeData.into());
        }
        if stake_data.account != *ctx.accounts.source_nft_account.key {
            return Err(PoolError::InvalidTokenAccount.into());
        }
        if stake_data.account == *ctx.accounts.dest_nft_account.key {
            return Err(PoolError::InvalidTokenAccount.into());
        }

        let pool_seeds = &[
            pool.rand.as_ref(),
            &[pool.bump],
        ];        
        spl_token_transfer(
            TokenTransferParams{
                source : ctx.accounts.source_nft_account.clone(),
                destination : ctx.accounts.dest_nft_account.clone(),
                authority : pool.to_account_info().clone(),
                authority_signer_seeds : pool_seeds,
                token_program : ctx.accounts.token_program.clone(),
                amount : 1,
            }
        )?;
        
        stake_data.unstaked = true;
        
        Ok(())
    }

    pub fn claim(
        ctx : Context<Claim>
        ) -> ProgramResult {
        let pool = &ctx.accounts.pool;
        let stake_data = &mut ctx.accounts.stake_data;
        let clock = Clock::from_account_info(&ctx.accounts.clock)?;
        if stake_data.owner != *ctx.accounts.owner.key {
            msg!("Not match owner");
            return Err(PoolError::InvalidStakeData.into());
        }
        if stake_data.pool != pool.key() {
            msg!("Not match pool");
            return Err(PoolError::InvalidStakeData.into());
        }
        if stake_data.withdrawn_number >= pool.withdrawable {
            msg!("already withdrawn all");
            return Err(PoolError::InvalidTime.into());
        }
        if pool.reward_account != *ctx.accounts.source_reward_account.key {
            msg!("Source reward account must be pool's reward account");
            return Err(PoolError::InvalidTokenAccount.into());
        }
        if pool.reward_account == *ctx.accounts.dest_reward_account.key {
            msg!("Dest reward account is not allowed to be pool's reward account");
            return Err(PoolError::InvalidTokenAccount.into());
        }

        let mut number = ((clock.unix_timestamp - stake_data.stake_time) / pool.period) as u8;
        if number > pool.withdrawable {
            number = pool.withdrawable;
        }

        let amount = pool.reward_amount * (number - stake_data.withdrawn_number) as u64;

        let pool_seeds = &[
            pool.rand.as_ref(),
            &[pool.bump],
        ];

        spl_token_transfer(
            TokenTransferParams{
                source : ctx.accounts.source_reward_account.clone(),
                destination : ctx.accounts.dest_reward_account.clone(),
                authority : pool.to_account_info().clone(),
                authority_signer_seeds : pool_seeds,
                token_program : ctx.accounts.token_program.clone(),
                amount : amount,
            }
        )?;

        stake_data.withdrawn_number = number;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut, signer)]
    owner : AccountInfo<'info>,   

    pool : ProgramAccount<'info,Pool>,

    #[account(mut)]
    stake_data : ProgramAccount<'info,StakeData>,

    #[account(mut,owner=spl_token::id())]
    source_reward_account : AccountInfo<'info>,

    #[account(mut,owner=spl_token::id())]
    dest_reward_account : AccountInfo<'info>,

    #[account(address=spl_token::id())]
    token_program : AccountInfo<'info>,

    clock : AccountInfo<'info>,     
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut, signer)]
    owner : AccountInfo<'info>,   

    pool : ProgramAccount<'info,Pool>,

    #[account(mut)]
    stake_data : ProgramAccount<'info,StakeData>,

    #[account(mut,owner=spl_token::id())]
    source_nft_account : AccountInfo<'info>,

    #[account(mut,owner=spl_token::id())]
    dest_nft_account : AccountInfo<'info>,

    #[account(address=spl_token::id())]
    token_program : AccountInfo<'info>,

    clock : AccountInfo<'info>,             
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut, signer)]
    owner : AccountInfo<'info>, 

    pool : ProgramAccount<'info,Pool>,

    #[account(init, payer=owner, space=8+STAKEDATA_SIZE)]
    stake_data : ProgramAccount<'info,StakeData>,

    #[account(mut,owner=spl_token::id())]
    nft_mint : AccountInfo<'info>,

    #[account(mut)]
    metadata : AccountInfo<'info>,

    #[account(mut,owner=spl_token::id())]
    source_nft_account : AccountInfo<'info>,

    #[account(mut,owner=spl_token::id())]
    dest_nft_account : AccountInfo<'info>,

    #[account(address=spl_token::id())]
    token_program : AccountInfo<'info>,

    system_program : Program<'info,System>,

    clock : AccountInfo<'info>,    
}

#[derive(Accounts)]
#[instruction(_bump : u8)]
pub struct InitPool<'info> {
    #[account(mut, signer)]
    owner : AccountInfo<'info>,

    #[account(init, seeds=[(*rand.key).as_ref()], bump=_bump, payer=owner, space=8+POOL_SIZE)]
    pool : ProgramAccount<'info, Pool>,

    rand : AccountInfo<'info>,

    #[account(owner=spl_token::id())]
    reward_mint : AccountInfo<'info>,

    #[account(owner=spl_token::id())]
    reward_account : AccountInfo<'info>,

    system_program : Program<'info,System>,
}

pub const POOL_SIZE : usize = 32 + 32 + 32 + 32 + 8 + 8 + 1 + 4 + MAX_SYMBOL_LENGTH + 1;
pub const STAKEDATA_SIZE : usize = 1 + 32 + 32 + 32 + 8 + 1;
pub const PERIOD : i64 = 24 * 60 * 60;

#[account]
pub struct Pool {
    pub owner : Pubkey,
    pub rand : Pubkey,
    pub reward_mint : Pubkey,
    pub reward_account : Pubkey,
    pub reward_amount : u64,
    pub period : i64,
    pub withdrawable : u8,
    pub stake_collection : String,
    pub bump : u8,
}

#[account]
pub struct StakeData {
    pub unstaked : bool,
    pub owner : Pubkey,
    pub pool : Pubkey,
    pub account : Pubkey,
    pub stake_time : i64,
    pub withdrawn_number : u8,
}

#[error]
pub enum PoolError {
    #[msg("Token mint to failed")]
    TokenMintToFailed,

    #[msg("Token set authority failed")]
    TokenSetAuthorityFailed,

    #[msg("Token transfer failed")]
    TokenTransferFailed,

    #[msg("Invalid token account")]
    InvalidTokenAccount,

    #[msg("Invalid token mint")]
    InvalidTokenMint,

    #[msg("Invalid metadata")]
    InvalidMetadata,

    #[msg("Invalid stakedata account")]
    InvalidStakeData,

    #[msg("Invalid time")]
    InvalidTime,

    #[msg("Invalid Period")]
    InvalidPeriod,

    #[msg("Already unstaked")]
    AlreadyUnstaked,
}