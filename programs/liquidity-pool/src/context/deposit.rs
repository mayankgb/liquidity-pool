use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token::{transfer_checked, TransferChecked}, token_interface::{Mint, TokenAccount, TokenInterface}};
use crate::{error::{DepositError, PoolError}, state::*};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub usdc_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub wrapped_sol_mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut, 
        associated_token::mint = usdc_mint, 
        associated_token::authority = signer, 
        associated_token::token_program = token_program
    )]
    pub user_usdc_ata: InterfaceAccount<'info,TokenAccount>,
    #[account(
        mut, 
        associated_token::mint = wrapped_sol_mint, 
        associated_token::authority = signer, 
        associated_token::token_program = token_program
    )]
    pub user_sol_ata: InterfaceAccount<'info,TokenAccount>,
    #[account(
        init_if_needed, 
        payer = signer, 
        space = 8 + User::INIT_SPACE, 
        seeds = [b"lp", signer.key().as_ref()],
        bump
    )]
    pub user_pda: Account<'info, User>,
    #[account(
        init_if_needed, 
        payer = signer, 
        space = 8 + Pool::INIT_SPACE, 
        seeds = [b"pool", usdc_mint.key().as_ref(), wrapped_sol_mint.key().as_ref()],
        bump
    )]
    pub pool_pda: Account<'info, Pool>,

    #[account(
        init_if_needed, 
        payer = signer, 
        associated_token::mint = usdc_mint, 
        associated_token::authority = pool_pda, 
        associated_token::token_program = token_program
    )]
    pub pool_usdc_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed, 
        payer = signer, 
        associated_token::mint = wrapped_sol_mint, 
        associated_token::authority = pool_pda, 
        associated_token::token_program = token_program
    )]
    pub pool_sol_ata: InterfaceAccount<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>
}


pub fn process_deposit(ctx: Context<Deposit>, usdc_amount: u64, wrapped_sol_amount: u64) -> Result<()> {

    require!( (usdc_amount > 0 || wrapped_sol_amount > 0), DepositError::ZeroAmountError );
    let pool_pda =&mut ctx.accounts.pool_pda;
    let user_pda = &mut ctx.accounts.user_pda;

    let is_pool_initialise = pool_pda.is_initialise;

    if !is_pool_initialise {
        user_pda.owner = ctx.accounts.signer.key();
        user_pda.sol_deposit = wrapped_sol_amount;
        user_pda.usdc_deposit = usdc_amount; 

        pool_pda.total_usdc_deposit = usdc_amount;
        pool_pda.total_sol_deposit = wrapped_sol_amount;
        pool_pda.bump = ctx.bumps.pool_pda;
        pool_pda.is_initialise = true;
        pool_pda.fees_collected_usdc = 0;
        pool_pda.liquidity_fees = 30;

        let product = (usdc_amount as u128).checked_mul(wrapped_sol_amount as u128).ok_or(PoolError::MathOverFlow)?;
        let user_shares = product.isqrt() as u64;

        user_pda.total_shares = user_shares;
        pool_pda.total_shares = user_shares;

        let usdc_cpi_accounts = TransferChecked {
            from: ctx.accounts.user_usdc_ata.to_account_info(), 
            to: ctx.accounts.pool_usdc_ata.to_account_info(), 
            mint:ctx.accounts.usdc_mint.to_account_info(),
            authority: ctx.accounts.signer.to_account_info()
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
             usdc_cpi_accounts);

        transfer_checked(cpi_ctx, usdc_amount, ctx.accounts.usdc_mint.decimals)?;

        let sol_cpi_accounts = TransferChecked {
            from: ctx.accounts.user_sol_ata.to_account_info(), 
            to: ctx.accounts.pool_sol_ata.to_account_info(), 
            mint: ctx.accounts.wrapped_sol_mint.to_account_info(), 
            authority: ctx.accounts.signer.to_account_info()
        };

        let sol_cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
                    sol_cpi_accounts);

        transfer_checked(sol_cpi_ctx, wrapped_sol_amount, ctx.accounts.wrapped_sol_mint.decimals)?;

        msg!("Pool is initiliased by signer {}", ctx.accounts.signer.key().to_string());
    }else { 

        let deposit_ratio = (usdc_amount as u128).checked_mul(100_000).ok_or(DepositError::MultiplyError)?.checked_div(wrapped_sol_amount as u128).ok_or(DepositError::DivisionError)? as u64;
        let pool_ratio = (pool_pda.total_usdc_deposit as u128).checked_mul(100_000).ok_or(DepositError::MultiplyError)?.checked_div(pool_pda.total_sol_deposit as u128).ok_or(DepositError::DivisionError)? as u64;
        

        let diff = if deposit_ratio > pool_ratio {
            deposit_ratio - pool_ratio
        } else {
            pool_ratio - deposit_ratio
        };

        require!(diff <= pool_ratio / 100, PoolError::ImbalancedDeposit);

        let usdc_shares = (usdc_amount as u128)
                            .checked_mul(pool_pda.total_shares as u128)
                            .ok_or(DepositError::MultiplyError)?
                            .checked_div(pool_pda.total_usdc_deposit as u128)
                            .ok_or(DepositError::DivisionError)?;
        let sol_shares = (wrapped_sol_amount as u128)
                            .checked_mul(pool_pda.total_shares as u128)
                            .ok_or(DepositError::MultiplyError)?
                            .checked_div(pool_pda.total_sol_deposit as u128)
                            .ok_or(DepositError::DivisionError)?;
        
         let new_shares = usdc_shares.min(sol_shares) as u64;
         require!(new_shares > 0, PoolError::ZeroShares);


        let usdc_cpi_accounts = TransferChecked {
            from: ctx.accounts.user_usdc_ata.to_account_info(), 
            to: ctx.accounts.pool_usdc_ata.to_account_info(), 
            mint:ctx.accounts.usdc_mint.to_account_info(),
            authority: ctx.accounts.signer.to_account_info()
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
             usdc_cpi_accounts);

        transfer_checked(cpi_ctx, usdc_amount, ctx.accounts.usdc_mint.decimals)?;

        let sol_cpi_accounts = TransferChecked {
            from: ctx.accounts.user_sol_ata.to_account_info(), 
            to: ctx.accounts.pool_sol_ata.to_account_info(), 
            mint: ctx.accounts.wrapped_sol_mint.to_account_info(), 
            authority: ctx.accounts.signer.to_account_info()
        };

        let sol_cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
                    sol_cpi_accounts);

        transfer_checked(sol_cpi_ctx, wrapped_sol_amount, ctx.accounts.wrapped_sol_mint.decimals)?;

      

        user_pda.sol_deposit += wrapped_sol_amount;
        user_pda.usdc_deposit += usdc_amount;
        user_pda.total_shares += new_shares;
        user_pda.owner = ctx.accounts.signer.key();

        pool_pda.total_shares += new_shares;
        pool_pda.total_sol_deposit += wrapped_sol_amount;
        pool_pda.total_usdc_deposit += usdc_amount;
    }

    Ok(())
}