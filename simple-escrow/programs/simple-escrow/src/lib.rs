use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint, Transfer as TokenTransfer, transfer};
use anchor_spl::associated_token::AssociatedToken;

declare_id!("BYH7nGfE4hekSgVevYbwGuiwymMCtRgX7ecWHshCmwmU");


#[program]
pub mod simple_escrow {
    use super::*;

    pub fn initialize_escrow(
        ctx: Context<InitializeEscrow>, 
        amount: u64, 
        receiver: Pubkey
    ) -> Result<()> {
        // Save escrow info
        let escrow = &mut ctx.accounts.escrow;
        escrow.initializer = ctx.accounts.initializer.key();
        escrow.receiver = receiver;
        escrow.mint = ctx.accounts.mint.key();
        escrow.amount = amount;
        escrow.bump = ctx.bumps.vault_authority;

        // Transfer tokens from initializer -> vault
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TokenTransfer {
                from: ctx.accounts.initializer_token_account.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.initializer.to_account_info(),
            }
        );
        transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn claim_escrow(ctx: Context<ClaimEscrow>) -> Result<()> {
        let escrow_key = ctx.accounts.escrow.key();
        let bump = ctx.accounts.escrow.bump;
        let seeds = &[b"vault", escrow_key.as_ref(), &[bump]];
        let signer = &[&seeds[..]];

        // Transfer tokens from vault -> receiver
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TokenTransfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.receiver_token_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            signer
        );
        transfer(cpi_ctx, ctx.accounts.escrow.amount)?;

        Ok(())
    }
}

#[account]
pub struct Escrow {
    pub initializer: Pubkey,
    pub receiver: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub bump: u8,
}

#[derive(Accounts)]
pub struct InitializeEscrow<'info> {
    #[account(
        init,
        payer = initializer,
        space = 8 + 32 + 32 + 32 + 8 + 1
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(mut)]
    pub initializer: Signer<'info>,

    #[account(mut)]
    pub initializer_token_account: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"vault", escrow.key().as_ref()],
        bump
    )]
    /// CHECK: PDA authority
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint,
        associated_token::authority = vault_authority
    )]
    pub vault: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct ClaimEscrow<'info> {
    #[account(mut, has_one = receiver)]
    pub escrow: Account<'info, Escrow>,

    #[account(
        seeds = [b"vault", escrow.key().as_ref()],
        bump = escrow.bump
    )]
    /// CHECK: PDA authority
    pub vault_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub receiver: Signer<'info>,

    #[account(mut)]
    pub receiver_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

