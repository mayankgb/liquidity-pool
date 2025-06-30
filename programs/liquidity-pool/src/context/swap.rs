
use anchor_lang::{prelude::*};

use anchor_spl::{associated_token::AssociatedToken, token::{transfer_checked, TransferChecked}, token_interface::{Mint, TokenAccount, TokenInterface}};

use crate::{ state::Pool};
use crate::error::{PoolError,DepositError};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub usdc_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub wrapped_sol_mint:InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub user_quote_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init_if_needed, 
        payer = signer , 
        associated_token::mint = base_mint, 
        associated_token::authority = signer,  
        associated_token::token_program = token_program
    )]
    pub user_base_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut, 
        constraint = (base_mint.key() != user_quote_ata.mint.key())
    )]
    pub base_mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        seeds = [b"pool", usdc_mint.key().as_ref(), wrapped_sol_mint.key().as_ref()],
        bump = pool_pda.bump
    )]
    pub pool_pda: Account<'info, Pool>,
    #[account(
        mut, 
        associated_token::mint = usdc_mint, 
        associated_token::authority = pool_pda, 
    )]
    pub pool_usdc_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut, 
        associated_token::mint = wrapped_sol_mint, 
        associated_token::authority = pool_pda, 
    )]
    pub pool_sol_ata: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>
}


pub fn process_swap(ctx: Context<Swap>, swap_amount: u64) -> Result<()> {

    let user_base_asset_key = ctx.accounts.user_base_ata.mint.key();
    let user_quote_asset_key = ctx.accounts.user_quote_ata.mint.key();
    let usdc_mint = ctx.accounts.usdc_mint.key();
    let wrapped_sol_mint = ctx.accounts.wrapped_sol_mint.key();
    let bump_pool = ctx.accounts.pool_pda.bump;
    let pool_pda =&mut  ctx.accounts.pool_pda;

    require!(swap_amount > 0, DepositError::ZeroAmountError);

    require!((user_base_asset_key != user_quote_asset_key), DepositError::InvalidAccountInputs);
    require!((user_base_asset_key == usdc_mint || user_base_asset_key == wrapped_sol_mint), DepositError::InvalidAccounts);
    require!((user_quote_asset_key == usdc_mint || user_quote_asset_key == wrapped_sol_mint), DepositError::InvalidAccounts);

    if user_quote_asset_key == usdc_mint {
        let total_usdc = pool_pda.total_usdc_deposit.checked_add(swap_amount).ok_or(DepositError::Underflow)? ;
        let total_sol = pool_pda.total_sol_deposit;
        let constant_product = total_sol as u128 * pool_pda.total_usdc_deposit as u128;
        let remaining_sol = constant_product.checked_div(total_usdc as u128).ok_or(DepositError::DivisionError)? as u64;
        let sol_to_be_transfered = total_sol.checked_sub(remaining_sol).ok_or(DepositError::Underflow)?;

        let transaction_fee  = swap_amount.checked_mul(pool_pda.liquidity_fees).unwrap().checked_div(10_000).unwrap();
        let required_usdc = swap_amount.checked_add(transaction_fee).ok_or(DepositError::OverFlow)?;

        let usdc_cpi_accounts = TransferChecked {
            from: ctx.accounts.user_quote_ata.to_account_info(),
            to: ctx.accounts.pool_usdc_ata.to_account_info(), 
            authority: ctx.accounts.signer.to_account_info(),
            mint: ctx.accounts.usdc_mint.to_account_info()
        };

        let usdc_cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            usdc_cpi_accounts);
        
        transfer_checked(usdc_cpi_ctx, required_usdc, ctx.accounts.usdc_mint.decimals)?;

        let seeds = [b"pool", usdc_mint.as_ref(), wrapped_sol_mint.as_ref(),&[bump_pool]];

        let sol_cpi_account = TransferChecked {
            from:ctx.accounts.pool_sol_ata.to_account_info(), 
            to: ctx.accounts.user_base_ata.to_account_info(), 
            mint: ctx.accounts.wrapped_sol_mint.to_account_info(),
            authority: pool_pda.to_account_info()
        };

        let signer_seeds: &[&[&[u8]]] = &[&seeds[..]];

        let sol_cpi = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            sol_cpi_account,
            signer_seeds
        );
        
        transfer_checked(sol_cpi, sol_to_be_transfered, ctx.accounts.wrapped_sol_mint.decimals)?;

        //update pool 
        pool_pda.total_sol_deposit = remaining_sol;
        pool_pda.total_usdc_deposit += swap_amount;
        pool_pda.fees_collected_usdc += transaction_fee

    }else if user_quote_asset_key == wrapped_sol_mint {

        let total_sol = pool_pda.total_sol_deposit;
        let total_usdc =pool_pda.total_usdc_deposit;

        let total_updated_sol = total_sol.checked_add(swap_amount).ok_or(DepositError::OverFlow)?;
        let constant_product = total_sol as u128 * total_usdc as u128;
        let required_usdc = constant_product.checked_div(total_updated_sol as u128).ok_or(DepositError::DivisionError)? as u64 - pool_pda.total_usdc_deposit; 
        let fee_required = ((required_usdc as f64 )* ((pool_pda.liquidity_fees as f64 / 100_00.0)) )as u64;
        let usdc_to_be_paid = required_usdc.checked_sub(fee_required).ok_or(DepositError::Underflow)? ;

        require!( pool_pda.total_usdc_deposit >= usdc_to_be_paid + fee_required, PoolError::InsufficientLiquidity);

        pool_pda.total_sol_deposit += swap_amount;
        pool_pda.total_usdc_deposit -= usdc_to_be_paid + fee_required;
        pool_pda.fees_collected_usdc += fee_required;

        let sol_cpi_accounts = TransferChecked {
            from: ctx.accounts.user_quote_ata.to_account_info(), 
            to: ctx.accounts.pool_sol_ata.to_account_info(), 
            mint: ctx.accounts.wrapped_sol_mint.to_account_info(),
            authority: ctx.accounts.signer.to_account_info() 
        };

        let sol_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            sol_cpi_accounts);
        
        transfer_checked(sol_ctx, swap_amount, ctx.accounts.wrapped_sol_mint.decimals)?;


        let usdc_cpi_accounts = TransferChecked {
            from: ctx.accounts.pool_usdc_ata.to_account_info(), 
            to: ctx.accounts.user_base_ata.to_account_info(), 
            mint: ctx.accounts.usdc_mint.to_account_info(), 
            authority: ctx.accounts.pool_pda.to_account_info()   
        };

        let seeds = [b"pool", usdc_mint.as_ref(), wrapped_sol_mint.as_ref(), &[bump_pool]];

        let signer_seeds: &[&[&[u8]]] = &[&seeds[..]];

        let usdc_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            usdc_cpi_accounts, 
            signer_seeds
        );
        transfer_checked(usdc_ctx, usdc_to_be_paid, ctx.accounts.usdc_mint.decimals)?;
    }

    Ok(())
}