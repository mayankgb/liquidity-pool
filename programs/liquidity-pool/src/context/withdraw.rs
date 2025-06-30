use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token::{transfer_checked, TransferChecked}, token_interface::{Mint, TokenAccount, TokenInterface}};

use crate::{error::DepositError, state::{Pool, User}};


#[derive(Accounts)]
pub struct WithDraw<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub usdc_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub wrapped_sol_mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        close = signer, 
        seeds = [b"lp", signer.key().as_ref()],
        bump
    )]
    pub user_pda: Account<'info, User>,
    #[account(
        init_if_needed, 
        payer = signer,
        associated_token::mint = usdc_mint,
        associated_token::authority = signer, 
        associated_token::token_program = token_program
    )]
    pub user_usdc_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init_if_needed, 
        payer = signer, 
        associated_token::mint = wrapped_sol_mint,
        associated_token::token_program = token_program,
        associated_token::authority = signer,
    )]
    pub user_sol_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut, 
        seeds = [b"pool", usdc_mint.key().as_ref(), wrapped_sol_mint.key().as_ref()],
        bump
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
    pub pool_wrapped_sol_ata: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>
}

pub fn process_withdraw(ctx: Context<WithDraw>) -> Result<()> {

    let user_pda = &mut ctx.accounts.user_pda; 
    let pool_pda =&mut ctx.accounts.pool_pda;
    let total_fee = pool_pda.fees_collected_usdc;
    let total_sol= pool_pda.total_sol_deposit;
    let total_usdc = pool_pda.total_usdc_deposit;
    let user_shares = user_pda.total_shares;
    let total_shares = pool_pda.total_shares; 
    let wrapped_sol_mint_key = ctx.accounts.wrapped_sol_mint.to_account_info().key();
    let usdc_mint_key = ctx.accounts.usdc_mint.to_account_info().key();

    let user_owned_portion = (user_shares as f64) / (total_shares as f64);

    let user_sol = ((total_sol as f64) * user_owned_portion) as u64;
    let user_usdc = ((total_usdc as f64) * user_owned_portion) as u64;
    let user_reward =( (total_fee as f64) * user_owned_portion) as u64;

    let tota_usdc_to_be_paid = user_usdc.checked_add(user_reward).ok_or(DepositError::OverFlow)?;

    let usdc_cpi_accounts = TransferChecked {
        from: ctx.accounts.pool_usdc_ata.to_account_info(), 
        to: ctx.accounts.user_usdc_ata.to_account_info(), 
        mint: ctx.accounts.usdc_mint.to_account_info(), 
        authority: pool_pda.to_account_info()
    };

    let seeds= [b"pool", usdc_mint_key.as_ref(), wrapped_sol_mint_key.as_ref(), &[pool_pda.bump]];
    let signer_seeds: &[&[&[u8]]] =&[&seeds[..]];

    let usdc_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        usdc_cpi_accounts,
        signer_seeds
    );

    transfer_checked(usdc_ctx, tota_usdc_to_be_paid, ctx.accounts.usdc_mint.decimals)?;

    let sol_cpi_accounts = TransferChecked {
        from: ctx.accounts.pool_wrapped_sol_ata.to_account_info(),
        to: ctx.accounts.user_sol_ata.to_account_info(), 
        mint: ctx.accounts.wrapped_sol_mint.to_account_info(), 
        authority: pool_pda.to_account_info() 
    };

    let sol_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        sol_cpi_accounts,
        signer_seeds
    );

    transfer_checked(sol_ctx, user_sol, ctx.accounts.wrapped_sol_mint.decimals)?;

    //update pool

    pool_pda.total_shares -= user_shares;
    pool_pda.total_sol_deposit -= user_sol; 
    pool_pda.total_usdc_deposit -= user_usdc;
    pool_pda.fees_collected_usdc -= user_reward;

    msg!("Withdraw successfull for {}", ctx.accounts.signer.key().to_string());

    Ok(())
}