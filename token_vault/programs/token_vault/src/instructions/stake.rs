use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

use crate::state::*;

fn harvest(user: &mut UserStake, vault: &Vault) {
    if user.amount == 0 { 
        user.reward_debt = vault.acc_reward_per_share; 
        return; 
    }
    let pending = (user.amount as u128)
        .saturating_mul(vault.acc_reward_per_share.saturating_sub(user.reward_debt))
        / Vault::SCALING;
    user.pending_rewards = user.pending_rewards.saturating_add(pending as u64);
    user.reward_debt = vault.acc_reward_per_share;
}

pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    {
        let vault = &mut ctx.accounts.vault;
        let user = &mut ctx.accounts.user_stake;

        vault.update_rewards(now);

        if user.amount == 0 {
            user.staker = ctx.accounts.authority.key();
            user.bump = ctx.bumps.user_stake;
        }

        harvest(user, vault);

        user.amount = user.amount.saturating_add(amount);
        vault.total_staked = vault.total_staked.saturating_add(amount);
        user.last_update = now;
        user.reward_debt = vault.acc_reward_per_share;
    } 
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
