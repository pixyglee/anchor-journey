use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::state::*;

pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
    let user_stake = &mut ctx.accounts.user_stake;
    let vault = &mut ctx.accounts.vault;
    let user_bump = ctx.bumps.user_stake;

    if user_stake.amount == 0 {
        user_stake.staker = ctx.accounts.authority.key();
        user_stake.last_update = Clock::get()?.unix_timestamp;
        user_stake.bump = user_bump;
    }

    user_stake.amount = user_stake.amount.saturating_add(amount);
    vault.total_staked = vault.total_staked.saturating_add(amount);

    token::transfer(ctx.accounts.into_transfer_to_vault_context(), amount)?;

    Ok(())
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(
        mut,
        seeds = [b"vault", vault.authority.as_ref()],
        bump,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        init_if_needed,
        payer = authority,
        space = 8 + UserStake::INIT_SPACE,
        seeds = [b"user-stake", authority.key().as_ref(), vault.key().as_ref()],
        bump
    )]
    pub user_stake: Account<'info, UserStake>,

    #[account(mut, token::authority = authority)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(mut, address = vault.token_account)]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> Stake<'info> {
    pub fn into_transfer_to_vault_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let cpi_accounts = token::Transfer {
            from: self.user_token_account.to_account_info(),
            to: self.vault_token_account.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}
