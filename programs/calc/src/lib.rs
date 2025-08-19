use anchor_lang::prelude::*;

declare_id!("91A9nBbWLmoYv7Z8GUd8BuEAC8bMGCuaa66tTY9qGybk");

#[program]
pub mod calc {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
