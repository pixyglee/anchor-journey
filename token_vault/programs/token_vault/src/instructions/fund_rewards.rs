use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::VaultError;

#[derive(Accounts)]
pub struct FundRewards<'info> {
    #[account(mut)]
    pub reward_vault: Account<'info, TokenAccount>, // vault ka reward pool

    #[account(mut)]
    pub admin_reward_ata: Account<'info, TokenAccount>, // admin ka ATA

    pub authority: Signer<'info>, // admin jo fund karega

    pub token_program: Program<'info, Token>,
}

pub fn fund_rewards(ctx: Context<FundRewards>, amount: u64) -> Result<()> {
    require!(amount > 0, VaultError::InsufficientFunds);

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.admin_reward_ata.to_account_info(),
                to: ctx.accounts.reward_vault.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        ),
        amount,
    )?;

    Ok(())
}
