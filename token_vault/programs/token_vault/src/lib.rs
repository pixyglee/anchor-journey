use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

// Replace with your actual program ID
declare_id!("EwbYP92VR6QeHSUpSrV7mGQhhqWNr8fE3z4uP8u4uvGj");

#[program]
pub mod token_vault {
    use super::*;

    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        bump: u8,
        authority_bump: u8,
    ) -> Result<()> {
        ctx.accounts.vault.set_inner(Vault {
            authority: ctx.accounts.payer.key(),
            token_account: ctx.accounts.token_account.key(),
            bump,
            authority_bump,
            is_locked: false,
            unlock_timestamp: 0,
        });
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let vault = &ctx.accounts.vault;

        // Check if vault is locked
        require!(!vault.is_locked, VaultError::VaultStillLocked);

        // Check if we have enough tokens
        require!(
            ctx.accounts.vault_token_account.amount >= amount,
            VaultError::InsufficientFunds
        );
        let vault_key = vault.key();
        // Create PDA signer seeds
        let authority_seed = &[
            b"authority",
            vault_key.as_ref(),
            &[vault.authority_bump],
        ];
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

    pub fn lock_vault(ctx: Context<LockVault>, unlock_timestamp: i64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.is_locked = true;
        vault.unlock_timestamp = unlock_timestamp;

        msg!("Vault locked until timestamp: {}", unlock_timestamp);
        Ok(())
    }

    pub fn unlock_vault(ctx: Context<UnlockVault>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        let clock = Clock::get()?;
        require!(
            clock.unix_timestamp >= vault.unlock_timestamp,
            VaultError::VaultStillLocked
        );
        vault.is_locked = false;
        vault.unlock_timestamp = 0;

        msg!("Vault unlocked successfully");
        Ok(())
    }
}

// -------------------------------------------------------------------------
// Account Structures
// -------------------------------------------------------------------------

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub authority: Pubkey,       // Who can control this vault
    pub token_account: Pubkey,   // The token account holding the funds
    pub bump: u8,                // PDA bump for the vault account
    pub authority_bump: u8,      // PDA bump for the vault authority
    pub is_locked: bool,         // Whether the vault is locked
    pub unlock_timestamp: i64,   // When the vault can be unlocked (0 = no time lock)
}

// -------------------------------------------------------------------------
// Error Codes
// -------------------------------------------------------------------------

#[error_code]
pub enum VaultError {
    #[msg("Vault is still locked")]
    VaultStillLocked,
    #[msg("Insufficient funds in vault")]
    InsufficientFunds,
    #[msg("Unauthorized access")]
    UnauthorizedAccess,
}

// -------------------------------------------------------------------------
// Instruction Contexts
// -------------------------------------------------------------------------

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

    /// CHECK: This account is safe because it's a PDA derived from the vault
    /// and its bump is verified and used for signing CPIs.
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
    pub token_account: Account<'info, TokenAccount>, // The token account for the vault

    pub mint: Account<'info, Mint>, // The mint of the token being stored

    #[account(mut)]
    pub payer: Signer<'info>, // The account paying for the transaction and account creation

    pub token_program: Program<'info, Token>, // The SPL Token program
    pub system_program: Program<'info, System>, // The System program for account creation
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump = vault.bump,
        has_one = authority // Ensures the signer is the vault's authority
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        token::authority = authority, // Ensures the signer owns this token account
    )]
    pub user_token_account: Account<'info, TokenAccount>, // The user's token account to deposit from

    #[account(
        mut,
        address = vault.token_account // Ensures this is the token account specified in the vault
    )]
    pub vault_token_account: Account<'info, TokenAccount>, // The vault's token account

    pub authority: Signer<'info>, // The user initiating the deposit (must be vault owner)
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump = vault.bump,
        has_one = authority // Ensures the signer is the vault's authority
    )]
    pub vault: Account<'info, Vault>,

    /// CHECK: This account is safe because it's a PDA derived from the vault
    /// and its bump is verified and used for signing CPIs.
    #[account(
        seeds = [b"authority", vault.key().as_ref()],
        bump = vault.authority_bump // Verify the authority PDA's bump
    )]
    pub vault_authority: UncheckedAccount<'info>, // PDA that owns the vault_token_account

    #[account(
        mut,
        token::authority = authority, // Ensures the signer owns this token account
    )]
    pub user_token_account: Account<'info, TokenAccount>, // The user's token account to withdraw to

    #[account(
        mut,
        address = vault.token_account // Ensures this is the token account specified in the vault
    )]
    pub vault_token_account: Account<'info, TokenAccount>, // The vault's token account

    pub authority: Signer<'info>, // The user initiating the withdrawal (must be vault owner)
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct LockVault<'info> {
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump = vault.bump,
        has_one = authority // Ensures only the vault authority can lock it
    )]
    pub vault: Account<'info, Vault>,

    pub authority: Signer<'info>, // The vault owner
}

#[derive(Accounts)]
pub struct UnlockVault<'info> {
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump = vault.bump,
        has_one = authority // Ensures only the vault authority can unlock it
    )]
    pub vault: Account<'info, Vault>,

    pub authority: Signer<'info>, // The vault owner
}