use anchor_lang::prelude::*;

// Import instruction and state modules
pub mod errors;
pub mod instructions;
pub mod state;

use errors::*;
use instructions::*;
use state::*;

declare_id!("EwbYP92VR6QeHSUpSrV7mGQhhqWNr8fE3z4uP8u4uvGj");

#[program]
pub mod token_vault {
    use super::*;

    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        vault_bump: u8,
        authority_bump: u8,
    ) -> Result<()> {
        instructions::initialize::initialize_vault(ctx, vault_bump, authority_bump)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::deposit(ctx, amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::withdraw::withdraw(ctx, amount)
    }

    pub fn lock_vault(ctx: Context<LockVault>, unlock_timestamp: i64) -> Result<()> {
        instructions::lock::lock_vault(ctx, unlock_timestamp)
    }

    pub fn unlock_vault(ctx: Context<UnlockVault>) -> Result<()> {
        instructions::unlock::unlock_vault(ctx)
    }

    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        instructions::stake::stake(ctx, amount)
    }

    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
        instructions::unstake::unstake(ctx, amount)
    }
}
