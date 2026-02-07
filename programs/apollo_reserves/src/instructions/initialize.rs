// programs/apollo_reserves/src/instructions/initialize.rs

use crate::errors::ReserveError;
use crate::events::ReservesInitialized;
use crate::state::{IbnrParams, ReserveConfig, ReserveState, RunoffState};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct InitializeReserves<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + ReserveConfig::INIT_SPACE,
        seeds = [ReserveConfig::SEED_PREFIX],
        bump
    )]
    pub reserve_config: Account<'info, ReserveConfig>,

    #[account(
        init,
        payer = authority,
        space = 8 + ReserveState::INIT_SPACE,
        seeds = [ReserveState::SEED_PREFIX],
        bump
    )]
    pub reserve_state: Account<'info, ReserveState>,

    #[account(
        init,
        payer = authority,
        space = 8 + RunoffState::INIT_SPACE,
        seeds = [RunoffState::SEED_PREFIX],
        bump
    )]
    pub runoff_state: Account<'info, RunoffState>,

    #[account(
        init,
        payer = authority,
        space = 8 + IbnrParams::INIT_SPACE,
        seeds = [IbnrParams::SEED_PREFIX],
        bump
    )]
    pub ibnr_params: Account<'info, IbnrParams>,

    /// USDC mint
    pub usdc_mint: Account<'info, anchor_spl::token::Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeReservesParams {
    pub governance_program: Pubkey,
    pub risk_engine_program: Pubkey,
    pub tier0_target_days: Option<u16>,
    pub tier1_target_days: Option<u16>,
    pub tier2_target_days: Option<u16>,
    pub reserve_margin_bps: Option<u16>,
    pub admin_load_bps: Option<u16>,
    pub initial_expected_daily_claims: u64,
}

pub fn handler(ctx: Context<InitializeReserves>, params: InitializeReservesParams) -> Result<()> {
    let clock = Clock::get()?;

    // Validate parameters
    let tier0_days = params
        .tier0_target_days
        .unwrap_or(ReserveConfig::DEFAULT_TIER0_DAYS);
    let tier1_days = params
        .tier1_target_days
        .unwrap_or(ReserveConfig::DEFAULT_TIER1_DAYS);
    let tier2_days = params
        .tier2_target_days
        .unwrap_or(ReserveConfig::DEFAULT_TIER2_DAYS);
    let reserve_margin = params
        .reserve_margin_bps
        .unwrap_or(ReserveConfig::DEFAULT_RESERVE_MARGIN_BPS);
    let admin_load = params
        .admin_load_bps
        .unwrap_or(ReserveConfig::DEFAULT_ADMIN_LOAD_BPS);

    require!(tier0_days > 0, ReserveError::InvalidTargetDays);
    require!(tier1_days > tier0_days, ReserveError::InvalidTargetDays);
    require!(tier2_days > tier1_days, ReserveError::InvalidTargetDays);
    require!(reserve_margin <= 5000, ReserveError::InvalidBasisPoints); // Max 50%
    require!(admin_load <= 2000, ReserveError::InvalidBasisPoints); // Max 20%

    // Initialize config
    let config = &mut ctx.accounts.reserve_config;
    config.authority = ctx.accounts.authority.key();
    config.usdc_mint = ctx.accounts.usdc_mint.key();
    config.tier0_target_days = tier0_days;
    config.tier1_target_days = tier1_days;
    config.tier2_target_days = tier2_days;
    config.min_coverage_ratio_bps = ReserveConfig::DEFAULT_MIN_COVERAGE_BPS;
    config.target_coverage_ratio_bps = ReserveConfig::DEFAULT_TARGET_COVERAGE_BPS;
    config.reserve_margin_bps = reserve_margin;
    config.admin_load_bps = admin_load;
    config.governance_program = params.governance_program;
    config.risk_engine_program = params.risk_engine_program;
    config.is_initialized = true;
    config.bump = ctx.bumps.reserve_config;
    config.reserved = vec![];

    // Initialize state
    let state = &mut ctx.accounts.reserve_state;
    state.tier0_balance = 0;
    state.tier1_balance = 0;
    state.tier2_balance = 0;
    state.runoff_balance = 0;
    state.expected_daily_claims = params.initial_expected_daily_claims;
    state.ibnr_usdc = 0;
    state.avg_reporting_lag_days = ReserveState::DEFAULT_REPORTING_LAG;
    state.development_factor_bps = ReserveState::DEFAULT_DEV_FACTOR_BPS;
    state.total_claims_paid = 0;
    state.total_contributions_received = 0;
    state.last_waterfall_at = 0;
    state.last_ibnr_computed_at = 0;
    state.current_coverage_ratio_bps = 0;
    state.bump = ctx.bumps.reserve_state;

    // Initialize run-off state
    let runoff = &mut ctx.accounts.runoff_state;
    runoff.target_balance = 0;
    runoff.estimated_legal_costs = 0;
    runoff.monthly_admin_costs = 0;
    runoff.winddown_months = RunoffState::DEFAULT_WINDDOWN_MONTHS;
    runoff.runoff_active = false;
    runoff.runoff_activated_at = 0;
    runoff.bump = ctx.bumps.runoff_state;

    // Initialize IBNR params
    let ibnr = &mut ctx.accounts.ibnr_params;
    ibnr.avg_daily_claims_30d = params.initial_expected_daily_claims;
    ibnr.avg_daily_claims_90d = params.initial_expected_daily_claims;
    ibnr.observed_reporting_lag = ReserveState::DEFAULT_REPORTING_LAG;
    ibnr.development_factor_bps = ReserveState::DEFAULT_DEV_FACTOR_BPS;
    ibnr.claims_std_dev = 0;
    ibnr.last_updated = clock.unix_timestamp;
    ibnr.sample_size = 0;
    ibnr.bump = ctx.bumps.ibnr_params;

    emit!(ReservesInitialized {
        authority: ctx.accounts.authority.key(),
        usdc_mint: ctx.accounts.usdc_mint.key(),
        tier0_target_days: tier0_days,
        tier1_target_days: tier1_days,
        tier2_target_days: tier2_days,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Set reserve targets (requires Actuarial Committee)
#[derive(Accounts)]
pub struct SetReserveTargets<'info> {
    #[account(
        mut,
        seeds = [ReserveConfig::SEED_PREFIX],
        bump = reserve_config.bump,
    )]
    pub reserve_config: Account<'info, ReserveConfig>,

    /// Must be DAO authority or pass through governance
    #[account(
        constraint = authority.key() == reserve_config.authority @ ReserveError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SetReserveTargetsParams {
    pub tier0_target_days: Option<u16>,
    pub tier1_target_days: Option<u16>,
    pub tier2_target_days: Option<u16>,
    pub min_coverage_ratio_bps: Option<u16>,
    pub target_coverage_ratio_bps: Option<u16>,
}

pub fn set_reserve_targets(
    ctx: Context<SetReserveTargets>,
    params: SetReserveTargetsParams,
) -> Result<()> {
    let config = &mut ctx.accounts.reserve_config;
    let clock = Clock::get()?;

    let old_t0 = config.tier0_target_days;
    let old_t1 = config.tier1_target_days;
    let old_t2 = config.tier2_target_days;

    if let Some(t0) = params.tier0_target_days {
        require!(t0 > 0, ReserveError::InvalidTargetDays);
        config.tier0_target_days = t0;
    }
    if let Some(t1) = params.tier1_target_days {
        require!(
            t1 > config.tier0_target_days,
            ReserveError::InvalidTargetDays
        );
        config.tier1_target_days = t1;
    }
    if let Some(t2) = params.tier2_target_days {
        require!(
            t2 > config.tier1_target_days,
            ReserveError::InvalidTargetDays
        );
        config.tier2_target_days = t2;
    }
    if let Some(min) = params.min_coverage_ratio_bps {
        require!(
            min >= 5000 && min <= 20000,
            ReserveError::InvalidBasisPoints
        );
        config.min_coverage_ratio_bps = min;
    }
    if let Some(target) = params.target_coverage_ratio_bps {
        require!(
            target > config.min_coverage_ratio_bps,
            ReserveError::InvalidBasisPoints
        );
        config.target_coverage_ratio_bps = target;
    }

    emit!(crate::events::ReserveTargetsUpdated {
        old_tier0_days: old_t0,
        new_tier0_days: config.tier0_target_days,
        old_tier1_days: old_t1,
        new_tier1_days: config.tier1_target_days,
        old_tier2_days: old_t2,
        new_tier2_days: config.tier2_target_days,
        updater: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
