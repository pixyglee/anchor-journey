use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct UserStake {
    pub staker: Pubkey,
    pub amount: u64,
    pub last_update: i64,
    pub bump: u8,
}