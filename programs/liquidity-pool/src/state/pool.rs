use anchor_lang::prelude::*;


#[account]
#[derive(InitSpace)]
pub struct Pool {
    pub total_usdc_deposit: u64, 
    pub total_sol_deposit: u64, 
    pub fees_collected_usdc: u64, 
    pub liquidity_fees: u64,
    pub total_shares: u64,
    pub bump: u8,
    pub is_initialise: bool
}