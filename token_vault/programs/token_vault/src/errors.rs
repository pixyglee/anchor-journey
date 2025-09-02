use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Vault is still locked")]
    VaultStillLocked,
    #[msg("Insufficient funds in vault")]
    InsufficientFunds,
    #[msg("Unauthorized access")]
    UnauthorizedAccess,
    #[msg("Insufficient staked balance")]
    InsufficientStake,
}