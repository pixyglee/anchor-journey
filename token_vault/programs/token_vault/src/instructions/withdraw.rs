use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::*;
use crate::state::*;

pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    let vault = &ctx.accounts.vault;
    require!(!vault.is_locked, VaultError::VaultStillLocked);
    require!(
        ctx.accounts.vault_token_account.amount >= amount,
        VaultError::InsufficientFunds
    );
    let vault_key = vault.key();
    let authority_seed = &[b"authority", vault_key.as_ref(), &[vault.authority_bump]];
    let signer = &[&authority_seed[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault_token_account.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.vault_authority.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    token::transfer(cpi_ctx, amount)?;
    Ok(())
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump = vault.bump,
        has_one = authority
    )]
    pub vault: Account<'info, Vault>,

    /// CHECK: This account is safe because it's a PDA derived from the vault
    /// and its bump is verified and used for signing CPIs.
    #[account(
        seeds = [b"authority", vault.key().as_ref()],
        bump = vault.authority_bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(mut, token::authority = authority)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(mut, address = vault.token_account)]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}
