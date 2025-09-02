use anchor_lang::prelude::*;

use crate::state::*;

pub fn lock_vault(ctx: Context<LockVault>, unlock_timestamp: i64) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.is_locked = true;
    vault.unlock_timestamp = unlock_timestamp;

    msg!("Vault locked until timestamp: {}", unlock_timestamp);
    Ok(())
}

#[derive(Accounts)]
pub struct LockVault<'info> {
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump = vault.bump,
        has_one = authority
    )]
    pub vault: Account<'info, Vault>,

    pub authority: Signer<'info>,
}
