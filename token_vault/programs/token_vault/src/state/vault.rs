use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub authority: Pubkey,
    pub token_account: Pubkey,
    pub bump: u8,
    pub authority_bump: u8,
    pub is_locked: bool,
    pub unlock_timestamp: i64,
    pub total_staked: u64,
}