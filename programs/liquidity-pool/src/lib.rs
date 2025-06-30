use anchor_lang::prelude::*;

mod state;
mod context;
mod error;

use state::*;
use context::*;
use error::*;

declare_id!("AH6xVywoqWvnPstLZVsvjYaaRFnQSLr8Dz2EbWgkAYx7");

#[program]
pub mod liquidity_pool {
    use super::*;

    pub fn deposit(ctx: Context<Deposit>, usdc_amount: u64, wrapped_sol_amount: u64) -> Result<()> {
        process_deposit(ctx, usdc_amount, wrapped_sol_amount)?;
        Ok(())
    }
    pub fn swap(ctx: Context<Swap> , swap_amount: u64) -> Result<()> {
        process_swap(ctx, swap_amount)?;
        Ok(())
    }

    pub fn withdraw(ctx: Context<WithDraw>) -> Result<()> {
        process_withdraw(ctx)?;
        Ok(())
    }
}