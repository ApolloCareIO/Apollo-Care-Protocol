// programs/apollo_membership/src/instructions/initialize.rs

use crate::events::GlobalConfigInitialized;
use crate::state::GlobalConfig;
use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

#[derive(Accounts)]
pub struct InitializeGlobalConfig<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + GlobalConfig::INIT_SPACE,
        seeds = [GlobalConfig::SEED_PREFIX],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    pub usdc_mint: Account<'info, Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeGlobalConfigParams {
    pub governance_program: Pubkey,
    pub risk_engine_program: Pubkey,
    pub reserves_program: Pubkey,
    pub default_waiting_period_days: Option<u16>,
    pub preexisting_waiting_days: Option<u16>,
}

pub fn handler(
    ctx: Context<InitializeGlobalConfig>,
    params: InitializeGlobalConfigParams,
) -> Result<()> {
    let clock = Clock::get()?;

    let config = &mut ctx.accounts.global_config;
    config.authority = ctx.accounts.authority.key();
    config.governance_program = params.governance_program;
    config.risk_engine_program = params.risk_engine_program;
    config.reserves_program = params.reserves_program;
    config.usdc_mint = ctx.accounts.usdc_mint.key();
    config.total_members = 0;
    config.active_members = 0;
    config.total_contributions = 0;
    config.default_waiting_period_days = params
        .default_waiting_period_days
        .unwrap_or(GlobalConfig::DEFAULT_WAITING_PERIOD);
    config.preexisting_waiting_days = params
        .preexisting_waiting_days
        .unwrap_or(GlobalConfig::DEFAULT_PREEXISTING_WAIT);
    config.persistency_discount_start_months = GlobalConfig::DEFAULT_PERSISTENCY_START;
    config.persistency_discount_bps = GlobalConfig::DEFAULT_PERSISTENCY_BPS;
    config.max_persistency_discount_bps = GlobalConfig::MAX_PERSISTENCY_BPS;
    config.enrollment_open = false;
    config.bump = ctx.bumps.global_config;

    emit!(GlobalConfigInitialized {
        authority: ctx.accounts.authority.key(),
        usdc_mint: ctx.accounts.usdc_mint.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
