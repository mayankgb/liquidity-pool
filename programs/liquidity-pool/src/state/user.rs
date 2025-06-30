use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct User {
    pub owner: Pubkey,
    pub usdc_deposit: u64, 
    pub sol_deposit: u64, 
    pub total_shares: u64,
}