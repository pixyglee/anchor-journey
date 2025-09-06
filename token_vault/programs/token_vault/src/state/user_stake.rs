use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct UserStake {
    pub staker: Pubkey,
    pub amount: u64,
    pub last_update: i64,
    pub bump: u8,

    pub reward_debt: u128,
    pub pending_rewards: u64,
}