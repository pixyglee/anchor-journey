use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::state::*;   
use crate::errors::*;  

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub vault: Account<'info, Vault>,

    #[account(mut, has_one = staker)]
    pub user_stake: Account<'info, UserStake>,
    
    pub staker: Signer<'info>,  

    /// CHECK: PDA authority for vault
    #[account(seeds = [b"authority", vault.key().as_ref()], bump = vault.authority_bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub reward_vault: Account<'info, TokenAccount>, // vault’s reward pool

    #[account(mut)]
    pub destination: Account<'info, TokenAccount>, // user’s ATA for reward mint

    pub token_program: Program<'info, Token>,
}

pub fn claim(ctx: Context<Claim>) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let user = &mut ctx.accounts.user_stake;
    let now = Clock::get()?.unix_timestamp;

    vault.update_rewards(now);

    let earned = (user.amount as u128)
        .saturating_mul(vault.acc_reward_per_share.saturating_sub(user.reward_debt))
        / Vault::SCALING;
    let payout = user.pending_rewards.saturating_add(earned as u64);

    require!(payout > 0, VaultError::InsufficientFunds);
    require!(ctx.accounts.reward_vault.amount >= payout, VaultError::InsufficientFunds);
    let vault_key = vault.key(); // keep it alive in a variable
    let seeds = &[b"authority", vault_key.as_ref(), &[vault.authority_bump]];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.reward_vault.to_account_info(),
        to: ctx.accounts.destination.to_account_info(),
        authority: ctx.accounts.vault_authority.to_account_info(),
    };

    token::transfer(
        CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts, signer),
        payout,
    )?;

    user.pending_rewards = 0;
    user.reward_debt = vault.acc_reward_per_share;
    Ok(())
}
