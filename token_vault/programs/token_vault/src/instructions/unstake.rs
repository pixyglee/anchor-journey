use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

use crate::errors::*;
use crate::state::*;

pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
    // clone values before mutable borrow to avoid conflicts
    let vault_key = ctx.accounts.vault.key();
    let authority_bump = ctx.accounts.vault.authority_bump;

    // mutable references
    let vault = &mut ctx.accounts.vault;
    let user = &mut ctx.accounts.user_stake;

    require!(user.amount >= amount, VaultError::InsufficientStake);

    let now = Clock::get()?.unix_timestamp;
    vault.update_rewards(now);

    // harvest rewards before balance changes
    let earned = (user.amount as u128)
        .saturating_mul(vault.acc_reward_per_share.saturating_sub(user.reward_debt))
        / Vault::SCALING;
    user.pending_rewards = user.pending_rewards.saturating_add(earned as u64);

    // seeds for PDA authority
    let seeds = &[b"authority", vault_key.as_ref(), &[authority_bump]];
    let signer = &[&seeds[..]];

    // build CPI transfer
    let cpi_accounts = token::Transfer {
        from: ctx.accounts.vault_token_account.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.vault_authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer,
    );

    token::transfer(cpi_ctx, amount)?;

    // update stake amounts
    user.amount = user.amount.saturating_sub(amount);
    vault.total_staked = vault.total_staked.saturating_sub(amount);
    user.reward_debt = vault.acc_reward_per_share;

    Ok(())
}



#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(
        mut,
        seeds = [b"vault", vault.authority.as_ref()],
        bump,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        seeds = [b"user-stake", authority.key().as_ref(), vault.key().as_ref()],
        bump,
        close = authority
    )]
    pub user_stake: Account<'info, UserStake>,

    #[account(mut, token::authority = authority)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(mut, address = vault.token_account)]
    pub vault_token_account: Account<'info, TokenAccount>,

    /// CHECK: PDA signer for vault
    #[account(
        seeds = [b"authority", vault.key().as_ref()],
        bump = vault.authority_bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}
