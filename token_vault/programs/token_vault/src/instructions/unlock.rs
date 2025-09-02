use anchor_lang::prelude::*;

use crate::errors::*;
use crate::state::*;

pub fn unlock_vault(ctx: Context<UnlockVault>) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp >= vault.unlock_timestamp,
        VaultError::VaultStillLocked
    );
    vault.is_locked = false;
    vault.unlock_timestamp = 0;

    msg!("Vault unlocked successfully");
    Ok(())
}

#[derive(Accounts)]
pub struct UnlockVault<'info> {
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump = vault.bump,
        has_one = authority
    )]
    pub vault: Account<'info, Vault>,

    pub authority: Signer<'info>,
}
