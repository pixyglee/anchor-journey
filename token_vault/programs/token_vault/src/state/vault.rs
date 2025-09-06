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

    pub reward_mint: Pubkey,
    pub reward_vault: Pubkey,
    pub reward_rate_per_second: u64,
    pub acc_reward_per_share: u128, 
    pub last_reward_ts: i64,
}

impl Vault {
    pub const SCALING: u128 = 1_000_000_000_000;

    pub fn update_rewards(&mut self, now: i64) {
        if self.total_staked == 0 {
            self.last_reward_ts = now;
            return;
        }
        let dt = now.saturating_sub(self.last_reward_ts) as u128;
        if dt == 0 { return; }

        let rewards = dt.saturating_mul(self.reward_rate_per_second as u128);
        let inc = rewards.saturating_mul(Self::SCALING) / (self.total_staked as u128);
        self.acc_reward_per_share = self.acc_reward_per_share.saturating_add(inc);
        self.last_reward_ts = now;
    }
}