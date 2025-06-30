use anchor_lang::prelude::*;

#[error_code]
pub enum DepositError {
    #[msg("Amount should be greater than 0")]
    ZeroAmountError,
    #[msg("Invalid Accounts Inputs")]
    InvalidAccountInputs,
    #[msg("Accounts does not belong to mint")]
    InvalidAccounts,
    #[msg("Airthmetic underflow")]
    Underflow, 
    #[msg("something went wrong")]
    DivisionError,
    #[msg("Airthmetic overflow")]
    OverFlow, 
    #[msg("Multiply Error")]
    MultiplyError
}


#[error_code]
pub enum PoolError {
    #[msg("MathOverflow")]
    MathOverFlow,
    #[msg("imbalance pool error")]
    ImbalancedDeposit,
    #[msg("pool error zero shares")]
    ZeroShares,
    #[msg("Insufficient liquidity")]
    InsufficientLiquidity
}

