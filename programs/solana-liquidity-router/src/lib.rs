use anchor_lang::prelude::*;

declare_id!("Ea63NeWVBCBrJuafvQy9JQJDbv5Q6K3MXbRFgiwFxfT");

#[program]
pub mod solana_liquidity_router {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
