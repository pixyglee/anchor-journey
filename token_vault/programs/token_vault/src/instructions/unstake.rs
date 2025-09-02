use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::*;
use crate::state::*;

pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
    let vault = &ctx.accounts.vault;
    let user_stake = &mut ctx.accounts.user_stake;

    require!(user_stake.amount >= amount, VaultError::InsufficientStake);

    user_stake.amount -= amount;
    let vault_key = vault.key();

    let seeds = &[b"authority", vault_key.as_ref(), &[vault.authority_bump]];
    let signer = &[&seeds[..]];

    let cpi_accounts = token::Transfer {
        from: ctx.accounts.vault_token_account.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.vault_authority.to_account_info(),
    };

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        ),
        amount,
    )?;

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
