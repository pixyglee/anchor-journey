use anchor_lang::prelude::*;

declare_id!("EFpkThxpS78297Lor9as1hW9pWa3My9kc1k1auiyS4b4");

const POINTS_PER_SOL_PER_DAY: u64 = 100_000;
const POINTS_PER_SOL_PAYOUT: u64 = 10_000_000_000;
const SECONDS_PER_DAY: u64 = 86_400;
const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

#[program]
pub mod staking_contract {
    use anchor_lang::system_program::{transfer, Transfer};

    use super::*;

    pub fn initialize_treasury(ctx: Context<InitializeTreasury>) -> Result<()> {
        let treasury = &mut ctx.accounts.treasury;
        treasury.admin = ctx.accounts.admin.key();
        treasury.bump = ctx.bumps.treasury;
        treasury.total_funded = 0;
        treasury.total_paid_out = 0;
        
        msg!("Treasury initialized with admin: {}", treasury.admin);
        Ok(())
    }

    pub fn fund_treasury(ctx: Context<FundTreasury>, amount: u64) -> Result<()> {
        require!(amount > 0, StakeError::InvalidAmount);
        
        let treasury = &mut ctx.accounts.treasury;
        
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.admin.to_account_info(),
                to: treasury.to_account_info(),
            },
        );
        
        transfer(cpi_context, amount)?;
        
        treasury.total_funded = treasury.total_funded
            .checked_add(amount)
            .ok_or(StakeError::Overflow)?;
        
        msg!("Treasury funded with {} lamports. Total funded: {}", amount, treasury.total_funded);
        Ok(())
    }

    pub fn create_pda_account(ctx: Context<CreatePdaAccount>) -> Result<()> {
        let pda_account = &mut ctx.accounts.pda_account;
        let clock = Clock::get()?;

        pda_account.owner = ctx.accounts.payer.key();
        pda_account.staked_amount = 0;
        pda_account.last_update_time = clock.unix_timestamp;
        pda_account.bump = ctx.bumps.pda_account;
        pda_account.total_points = 0;

        msg!("PDA account created successfully for user: {}", pda_account.owner);
        Ok(())
    }

    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        require!(amount > 0, StakeError::InvalidAmount);

        let pda_account = &mut ctx.accounts.pda_account;
        let clock = Clock::get()?;

        update_points(pda_account, clock.unix_timestamp)?;

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: pda_account.to_account_info(),
            },
        );

        transfer(cpi_context, amount)?;

        pda_account.staked_amount = pda_account
            .staked_amount
            .checked_add(amount)
            .ok_or(StakeError::Overflow)?;

        msg!(
            "Staked {} lamports. Total staked: {}, Total points: {}",
            amount,
            pda_account.staked_amount,
            pda_account.total_points
        );
        Ok(())
    }

    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
        require!(amount > 0, StakeError::InvalidAmount);

        let pda_account = &mut ctx.accounts.pda_account;
        let clock = Clock::get()?;

        require!(
            pda_account.staked_amount >= amount,
            StakeError::InsufficientStake
        );

        update_points(pda_account, clock.unix_timestamp)?;

        **pda_account.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.user.to_account_info().try_borrow_mut_lamports()? += amount;

        pda_account.staked_amount = pda_account
            .staked_amount
            .checked_sub(amount)
            .ok_or(StakeError::Underflow)?;

        msg!(
            "Unstaked {} lamports. Remaining staked: {}, Total points: {}",
            amount,
            pda_account.staked_amount,
            pda_account.total_points
        );

        Ok(())
    }

    pub fn claim_points(ctx: Context<ClaimPoints>) -> Result<()> {
        let pda_account = &mut ctx.accounts.pda_account;
        let clock = Clock::get()?;
        
        update_points(pda_account, clock.unix_timestamp)?;
        
        let claimable_points = pda_account.total_points;
        
        require!(claimable_points > 0, StakeError::NoPointsToClaim);
        
        msg!("User claimed {} points", claimable_points);
        
        pda_account.total_points = 0;
        
        Ok(())
    }

    pub fn convert_points_to_sol(
        ctx: Context<ConvertPointsToSol>, 
        points_to_convert: u64
    ) -> Result<()> {
        require!(points_to_convert > 0, StakeError::InvalidAmount);
        
        let pda_account = &mut ctx.accounts.pda_account;
        let treasury = &mut ctx.accounts.treasury;
        let clock = Clock::get()?;
        
        require!(!treasury.paused, StakeError::ConversionsPaused);
        
        update_points(pda_account, clock.unix_timestamp)?;
        
        require!(
            pda_account.total_points >= points_to_convert,
            StakeError::InsufficientPoints
        );
        
        let sol_payout = points_to_convert
            .checked_mul(LAMPORTS_PER_SOL)
            .ok_or(StakeError::Overflow)?
            .checked_div(POINTS_PER_SOL_PAYOUT)
            .ok_or(StakeError::DivisionByZero)?;
        
        require!(sol_payout > 0, StakeError::InsufficientPointsForPayout);
        
        let treasury_balance = treasury.to_account_info().lamports();
        let rent_exemption = Rent::get()?.minimum_balance(treasury.to_account_info().data_len());
        let available_balance = treasury_balance.checked_sub(rent_exemption).unwrap_or(0);
        
        require!(
            available_balance >= sol_payout,
            StakeError::InsufficientTreasuryFunds
        );
        
        **treasury.to_account_info().try_borrow_mut_lamports()? -= sol_payout;
        **ctx.accounts.user.to_account_info().try_borrow_mut_lamports()? += sol_payout;
        
        treasury.total_paid_out = treasury.total_paid_out
            .checked_add(sol_payout)
            .ok_or(StakeError::Overflow)?;
        
        pda_account.total_points = pda_account.total_points
            .checked_sub(points_to_convert)
            .ok_or(StakeError::Underflow)?;
        
        msg!(
            "Converted {} points to {} SOL ({} lamports). Remaining points: {}",
            points_to_convert,
            sol_payout as f64 / LAMPORTS_PER_SOL as f64,
            sol_payout,
            pda_account.total_points
        );
    
        let remaining_balance = treasury.to_account_info().lamports();
        let remaining_sol = remaining_balance as f64 / LAMPORTS_PER_SOL as f64;
    
        if remaining_sol < 10.0 {
            msg!("🚨 TREASURY ALERT: Only {:.1} SOL remaining! Refund urgently!", remaining_sol);
        } else if remaining_sol < 25.0 {
            msg!("⚠️  TREASURY WARNING: {:.1} SOL remaining. Consider refunding soon.", remaining_sol);
        }
        
        Ok(())
    }
    
    pub fn get_points(ctx: Context<GetPoints>) -> Result<()> {
        let pda_account = &ctx.accounts.pda_account;
        let clock = Clock::get()?;
        
        let time_elapsed = clock.unix_timestamp.checked_sub(pda_account.last_update_time)
            .ok_or(StakeError::InvalidTimestamp)? as u64;
        
        let new_points = if pda_account.staked_amount > 0 && time_elapsed > 0 {
            calculate_points_earned(pda_account.staked_amount, time_elapsed)?
        } else {
            0
        };
        
        let current_total_points = pda_account.total_points.checked_add(new_points)
            .ok_or(StakeError::Overflow)?;
        
        msg!(
            "Current points: {}, Staked amount: {} SOL, Time since last update: {} seconds", 
            current_total_points,
            pda_account.staked_amount as f64 / LAMPORTS_PER_SOL as f64,
            time_elapsed
        );
        
        Ok(())
    }

    pub fn get_treasury_info(ctx: Context<GetTreasuryInfo>) -> Result<()> {
        let treasury = &ctx.accounts.treasury;
        let balance = treasury.to_account_info().lamports();
        let rent_exemption = Rent::get()?.minimum_balance(treasury.to_account_info().data_len());
        let available_balance = balance.checked_sub(rent_exemption).unwrap_or(0);
        
        msg!(
            "Treasury - Balance: {} SOL, Available: {} SOL, Total Funded: {} SOL, Total Paid Out: {} SOL",
            balance as f64 / LAMPORTS_PER_SOL as f64,
            available_balance as f64 / LAMPORTS_PER_SOL as f64,
            treasury.total_funded as f64 / LAMPORTS_PER_SOL as f64,
            treasury.total_paid_out as f64 / LAMPORTS_PER_SOL as f64
        );
        
        Ok(())
    }
}

fn update_points(pda_account: &mut StakeAccount, current_time: i64) -> Result<()> {
    let time_elapsed = current_time
        .checked_sub(pda_account.last_update_time)
        .ok_or(StakeError::InvalidTimestamp)? as u64;

    if time_elapsed > 0 && pda_account.staked_amount > 0 {
        let new_points = calculate_points_earned(pda_account.staked_amount, time_elapsed)?;
        pda_account.total_points = pda_account
            .total_points
            .checked_add(new_points)
            .ok_or(StakeError::Overflow)?;
    }

    pda_account.last_update_time = current_time;

    Ok(())
}

fn calculate_points_earned(staked_amount: u64, time_elapsed_seconds: u64) -> Result<u64> {
    let points = (staked_amount as u128)
        .checked_mul(time_elapsed_seconds as u128)
        .ok_or(StakeError::Overflow)?
        .checked_mul(POINTS_PER_SOL_PER_DAY as u128)
        .ok_or(StakeError::Overflow)?
        .checked_div(LAMPORTS_PER_SOL as u128)
        .ok_or(StakeError::DivisionByZero)?
        .checked_div(SECONDS_PER_DAY as u128)
        .ok_or(StakeError::DivisionByZero)?;

    if points > u64::MAX as u128 {
        return Err(StakeError::Overflow.into());
    }

    Ok(points as u64)
}

pub fn pause_conversions(ctx: Context<AdminOnly>) -> Result<()> {
    let treasury = &mut ctx.accounts.treasury;
    treasury.paused = true;
    msg!("⏸️ Point conversions PAUSED by admin");
    Ok(())
}

pub fn unpause_conversions(ctx: Context<AdminOnly>) -> Result<()> {
    let treasury = &mut ctx.accounts.treasury;
    treasury.paused = false;
    msg!("▶️ Point conversions RESUMED by admin");
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeTreasury<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 8 + 8 + 1,
        seeds = [b"treasury"],
        bump
    )]
    pub treasury: Account<'info, Treasury>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FundTreasury<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"treasury"],
        bump = treasury.bump,
        constraint = treasury.admin == admin.key() @ StakeError::Unauthorized
    )]
    pub treasury: Account<'info, Treasury>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreatePdaAccount<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 8 + 8 + 8 + 1,
        seeds = [b"client", payer.key.as_ref()],
        bump
    )]
    pub pda_account: Account<'info, StakeAccount>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"client", user.key.as_ref()],
        bump = pda_account.bump,
        constraint = pda_account.owner == user.key() @ StakeError::Unauthorized
    )]
    pub pda_account: Account<'info, StakeAccount>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"client", user.key.as_ref()],
        bump = pda_account.bump,
        constraint = pda_account.owner == user.key() @ StakeError::Unauthorized
    )]
    pub pda_account: Account<'info, StakeAccount>,
}

#[derive(Accounts)]
pub struct ClaimPoints<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"client", user.key().as_ref()],
        bump = pda_account.bump,
        constraint = pda_account.owner == user.key() @ StakeError::Unauthorized 
    )]
    pub pda_account: Account<'info, StakeAccount>
}

#[derive(Accounts)]
pub struct ConvertPointsToSol<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"client", user.key().as_ref()],
        bump = pda_account.bump,
        constraint = pda_account.owner == user.key() @ StakeError::Unauthorized
    )]
    
    pub pda_account: Account<'info, StakeAccount>,
    
    #[account(
        mut,
        seeds = [b"treasury"],
        bump = treasury.bump
    )]
    pub treasury: Account<'info, Treasury>,
}

#[derive(Accounts)]
pub struct GetPoints<'info> {
    pub user: Signer<'info>,
    
    #[account(
        seeds = [b"client", user.key().as_ref()],
        bump = pda_account.bump,
        constraint = pda_account.owner == user.key() @ StakeError::Unauthorized
    )]
    pub pda_account: Account<'info, StakeAccount>,
}

#[derive(Accounts)]
pub struct GetTreasuryInfo<'info> {
    #[account(
        seeds = [b"treasury"],
        bump = treasury.bump
    )]
    pub treasury: Account<'info, Treasury>,
}

#[account]
pub struct StakeAccount {
    pub owner: Pubkey,
    pub staked_amount: u64,
    pub total_points: u64,
    pub last_update_time: i64,
    pub bump: u8,
}

#[account]
pub struct Treasury {
    pub admin: Pubkey,
    pub total_funded: u64,
    pub total_paid_out: u64,
    pub bump: u8,
    pub paused: bool,
}

#[derive(Accounts)]
pub struct AdminOnly<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"treasury"],
        bump = treasury.bump,
        constraint = treasury.admin == admin.key() @ StakeError::Unauthorized
    )]
    pub treasury: Account<'info, Treasury>,
}

#[error_code]
pub enum StakeError {
    #[msg("Unauthorized access")]
    Unauthorized,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Arithmetic underflow")]
    Underflow,
    #[msg("Division by zero")]
    DivisionByZero,
    #[msg("Invalid timestamp")]
    InvalidTimestamp,
    #[msg("Amount must be greater than 0")]
    InvalidAmount,
    #[msg("Insufficient staked amount")]
    InsufficientStake,
    #[msg("No points available to claim")]
    NoPointsToClaim,
    #[msg("Insufficient points")]
    InsufficientPoints,
    #[msg("Insufficient points for minimum payout")]
    InsufficientPointsForPayout,
    #[msg("Treasury has insufficient funds")]
    InsufficientTreasuryFunds,
    #[msg("Point conversions are temporarily paused")]
    ConversionsPaused,
}