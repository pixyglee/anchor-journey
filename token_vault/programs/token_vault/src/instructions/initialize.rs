use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

use crate::state::*;

pub fn initialize_vault(
    ctx: Context<InitializeVault>,
    _bump: u8,
    authority_bump: u8,
    reward_rate_per_second: u64, 
) -> Result<()> {
    // The _bump parameter is used to accept the vault bump, which
    // is automatically derived by Anchor. We use _ to indicate it's not
    // directly used in this function, but is necessary for the instruction.
    // The authority_bump parameter is what we actually need.
    let now = Clock::get()?.unix_timestamp;  
    
    ctx.accounts.vault.set_inner(Vault {
        authority: ctx.accounts.payer.key(),
        token_account: ctx.accounts.token_account.key(),
        bump: ctx.bumps.vault,
        authority_bump, // Correctly setting the authority_bump
        is_locked: false,
        unlock_timestamp: 0,
        total_staked: 0,

        reward_mint: ctx.accounts.reward_mint.key(),
        reward_vault: ctx.accounts.reward_vault.key(),
        reward_rate_per_second,
        acc_reward_per_share: 0,
        last_reward_ts: now,
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(bump: u8, authority_bump: u8)]
pub struct InitializeVault<'info> {
    #[account(
        init,
        payer = payer,
        seeds = [b"vault", payer.key().as_ref()],
        bump,
        space = 8 + Vault::INIT_SPACE
    )]
    pub vault: Account<'info, Vault>,

    /// CHECK: PDA signer for stake & reward vaults
    #[account(
        seeds = [b"authority", vault.key().as_ref()],
        bump = authority_bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        token::mint = mint,
        token::authority = vault_authority,
    )]
    pub token_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        token::mint = reward_mint,
        token::authority = vault_authority,
    )]
    pub reward_vault: Account<'info, TokenAccount>,
    pub reward_mint: Account<'info, Mint>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
