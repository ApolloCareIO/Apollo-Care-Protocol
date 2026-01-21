// programs/apollo_reserves/src/instructions/ibnr.rs

use anchor_lang::prelude::*;
use crate::state::{ReserveConfig, ReserveState, IbnrParams, RunoffState};
use crate::errors::ReserveError;
use crate::events::{IbnrUpdated, ExpectedClaimsUpdated, RunoffFunded, RunoffModeActivated};

/// Compute and update IBNR reserve
/// IBNR = (Avg Daily Claims × Reporting Lag Days) × Development Factor
#[derive(Accounts)]
pub struct ComputeIbnr<'info> {
    #[account(
        seeds = [ReserveConfig::SEED_PREFIX],
        bump = reserve_config.bump,
    )]
    pub reserve_config: Account<'info, ReserveConfig>,

    #[account(
        mut,
        seeds = [ReserveState::SEED_PREFIX],
        bump = reserve_state.bump,
    )]
    pub reserve_state: Account<'info, ReserveState>,

    #[account(
        seeds = [IbnrParams::SEED_PREFIX],
        bump = ibnr_params.bump,
    )]
    pub ibnr_params: Account<'info, IbnrParams>,

    /// Authorized updater (Actuarial Committee or system)
    pub updater: Signer<'info>,
}

pub fn compute_ibnr(ctx: Context<ComputeIbnr>) -> Result<()> {
    let clock = Clock::get()?;
    let state = &mut ctx.accounts.reserve_state;
    let params = &ctx.accounts.ibnr_params;

    let old_ibnr = state.ibnr_usdc;

    // Use 30-day average for more responsive IBNR
    let avg_daily = params.avg_daily_claims_30d;

    // Compute IBNR: avg_daily * lag_days * (dev_factor / 10000)
    let base = avg_daily.saturating_mul(params.observed_reporting_lag as u64);
    let new_ibnr = base
        .saturating_mul(params.development_factor_bps as u64)
        .checked_div(10000)
        .unwrap_or(0);

    state.ibnr_usdc = new_ibnr;
    state.avg_reporting_lag_days = params.observed_reporting_lag;
    state.development_factor_bps = params.development_factor_bps;
    state.last_ibnr_computed_at = clock.unix_timestamp;

    emit!(IbnrUpdated {
        old_ibnr,
        new_ibnr,
        avg_daily_claims: avg_daily,
        reporting_lag_days: params.observed_reporting_lag,
        development_factor_bps: params.development_factor_bps,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Update expected claims (daily average)
#[derive(Accounts)]
pub struct UpdateExpectedClaims<'info> {
    #[account(
        seeds = [ReserveConfig::SEED_PREFIX],
        bump = reserve_config.bump,
    )]
    pub reserve_config: Account<'info, ReserveConfig>,

    #[account(
        mut,
        seeds = [ReserveState::SEED_PREFIX],
        bump = reserve_state.bump,
    )]
    pub reserve_state: Account<'info, ReserveState>,

    #[account(
        mut,
        seeds = [IbnrParams::SEED_PREFIX],
        bump = ibnr_params.bump,
    )]
    pub ibnr_params: Account<'info, IbnrParams>,

    /// Must be authorized (Actuarial Committee)
    #[account(
        constraint = updater.key() == reserve_config.authority @ ReserveError::Unauthorized
    )]
    pub updater: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateExpectedClaimsParams {
    pub avg_daily_claims_30d: u64,
    pub avg_daily_claims_90d: u64,
    pub observed_reporting_lag: Option<u16>,
    pub development_factor_bps: Option<u16>,
    pub claims_std_dev: Option<u64>,
    pub sample_size: Option<u32>,
}

pub fn update_expected_claims(
    ctx: Context<UpdateExpectedClaims>,
    params: UpdateExpectedClaimsParams,
) -> Result<()> {
    let clock = Clock::get()?;
    let state = &mut ctx.accounts.reserve_state;
    let ibnr = &mut ctx.accounts.ibnr_params;

    require!(params.avg_daily_claims_30d > 0, ReserveError::ZeroExpectedClaims);

    let old_expected = state.expected_daily_claims;

    // Update state
    state.expected_daily_claims = params.avg_daily_claims_30d;

    // Update IBNR params
    ibnr.avg_daily_claims_30d = params.avg_daily_claims_30d;
    ibnr.avg_daily_claims_90d = params.avg_daily_claims_90d;

    if let Some(lag) = params.observed_reporting_lag {
        require!(lag > 0, ReserveError::InvalidIbnrParams);
        ibnr.observed_reporting_lag = lag;
    }

    if let Some(dev_factor) = params.development_factor_bps {
        require!(dev_factor >= 10000, ReserveError::InvalidDevFactor); // Must be >= 1.0
        ibnr.development_factor_bps = dev_factor;
    }

    if let Some(std_dev) = params.claims_std_dev {
        ibnr.claims_std_dev = std_dev;
    }

    if let Some(sample) = params.sample_size {
        ibnr.sample_size = sample;
    }

    ibnr.last_updated = clock.unix_timestamp;

    emit!(ExpectedClaimsUpdated {
        old_expected_daily: old_expected,
        new_expected_daily: params.avg_daily_claims_30d,
        updater: ctx.accounts.updater.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Update IBNR parameters directly (Actuarial Committee gated)
#[derive(Accounts)]
pub struct UpdateIbnrParams<'info> {
    #[account(
        seeds = [ReserveConfig::SEED_PREFIX],
        bump = reserve_config.bump,
    )]
    pub reserve_config: Account<'info, ReserveConfig>,

    #[account(
        mut,
        seeds = [IbnrParams::SEED_PREFIX],
        bump = ibnr_params.bump,
    )]
    pub ibnr_params: Account<'info, IbnrParams>,

    #[account(
        constraint = authority.key() == reserve_config.authority @ ReserveError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn update_ibnr_params(
    ctx: Context<UpdateIbnrParams>,
    reporting_lag: u16,
    development_factor_bps: u16,
) -> Result<()> {
    require!(reporting_lag > 0, ReserveError::InvalidIbnrParams);
    require!(development_factor_bps >= 10000, ReserveError::InvalidDevFactor);

    let clock = Clock::get()?;
    let params = &mut ctx.accounts.ibnr_params;

    params.observed_reporting_lag = reporting_lag;
    params.development_factor_bps = development_factor_bps;
    params.last_updated = clock.unix_timestamp;

    Ok(())
}

/// Fund the run-off reserve
#[derive(Accounts)]
pub struct FundRunoffReserve<'info> {
    #[account(
        seeds = [ReserveConfig::SEED_PREFIX],
        bump = reserve_config.bump,
    )]
    pub reserve_config: Account<'info, ReserveConfig>,

    #[account(
        mut,
        seeds = [ReserveState::SEED_PREFIX],
        bump = reserve_state.bump,
    )]
    pub reserve_state: Account<'info, ReserveState>,

    #[account(
        seeds = [RunoffState::SEED_PREFIX],
        bump = runoff_state.bump,
    )]
    pub runoff_state: Account<'info, RunoffState>,

    #[account(
        seeds = [crate::state::VaultAuthority::SEED_PREFIX],
        bump = vault_authority.bump,
    )]
    pub vault_authority: Account<'info, crate::state::VaultAuthority>,

    #[account(
        mut,
        constraint = source.mint == reserve_config.usdc_mint @ ReserveError::InvalidMint
    )]
    pub source: Account<'info, anchor_spl::token::TokenAccount>,

    #[account(
        mut,
        constraint = runoff_vault.key() == vault_authority.runoff_vault @ ReserveError::InvalidVaultConfig
    )]
    pub runoff_vault: Account<'info, anchor_spl::token::TokenAccount>,

    pub funder: Signer<'info>,

    pub token_program: Program<'info, anchor_spl::token::Token>,
}

pub fn fund_runoff_reserve(ctx: Context<FundRunoffReserve>, amount: u64) -> Result<()> {
    require!(amount > 0, ReserveError::ZeroAmount);

    let clock = Clock::get()?;

    anchor_spl::token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.source.to_account_info(),
                to: ctx.accounts.runoff_vault.to_account_info(),
                authority: ctx.accounts.funder.to_account_info(),
            },
        ),
        amount,
    )?;

    let state = &mut ctx.accounts.reserve_state;
    state.runoff_balance = state.runoff_balance.saturating_add(amount);

    emit!(RunoffFunded {
        amount,
        new_balance: state.runoff_balance,
        target_balance: ctx.accounts.runoff_state.target_balance,
        funder: ctx.accounts.funder.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Set run-off state parameters
#[derive(Accounts)]
pub struct SetRunoffParams<'info> {
    #[account(
        seeds = [ReserveConfig::SEED_PREFIX],
        bump = reserve_config.bump,
    )]
    pub reserve_config: Account<'info, ReserveConfig>,

    #[account(
        seeds = [ReserveState::SEED_PREFIX],
        bump = reserve_state.bump,
    )]
    pub reserve_state: Account<'info, ReserveState>,

    #[account(
        mut,
        seeds = [RunoffState::SEED_PREFIX],
        bump = runoff_state.bump,
    )]
    pub runoff_state: Account<'info, RunoffState>,

    #[account(
        constraint = authority.key() == reserve_config.authority @ ReserveError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SetRunoffParamsInput {
    pub estimated_legal_costs: Option<u64>,
    pub monthly_admin_costs: Option<u64>,
    pub winddown_months: Option<u8>,
}

pub fn set_runoff_params(ctx: Context<SetRunoffParams>, params: SetRunoffParamsInput) -> Result<()> {
    let runoff = &mut ctx.accounts.runoff_state;
    let reserve_state = &ctx.accounts.reserve_state;

    if let Some(legal) = params.estimated_legal_costs {
        runoff.estimated_legal_costs = legal;
    }
    if let Some(admin) = params.monthly_admin_costs {
        runoff.monthly_admin_costs = admin;
    }
    if let Some(months) = params.winddown_months {
        runoff.winddown_months = months;
    }

    // Recalculate target balance
    // 180 days IBNR + admin costs + legal costs
    let ibnr_180d = reserve_state.expected_daily_claims.saturating_mul(180);
    runoff.target_balance = runoff.required_runoff_reserve(ibnr_180d);

    Ok(())
}

/// Activate run-off mode (protocol wind-down)
#[derive(Accounts)]
pub struct ActivateRunoff<'info> {
    #[account(
        seeds = [ReserveConfig::SEED_PREFIX],
        bump = reserve_config.bump,
    )]
    pub reserve_config: Account<'info, ReserveConfig>,

    #[account(
        seeds = [ReserveState::SEED_PREFIX],
        bump = reserve_state.bump,
    )]
    pub reserve_state: Account<'info, ReserveState>,

    #[account(
        mut,
        seeds = [RunoffState::SEED_PREFIX],
        bump = runoff_state.bump,
        constraint = !runoff_state.runoff_active @ ReserveError::RunoffAlreadyActive
    )]
    pub runoff_state: Account<'info, RunoffState>,

    /// Requires DAO authority
    #[account(
        constraint = authority.key() == reserve_config.authority @ ReserveError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn activate_runoff(ctx: Context<ActivateRunoff>) -> Result<()> {
    let clock = Clock::get()?;
    let runoff = &mut ctx.accounts.runoff_state;
    let reserve_state = &ctx.accounts.reserve_state;

    runoff.runoff_active = true;
    runoff.runoff_activated_at = clock.unix_timestamp;

    emit!(RunoffModeActivated {
        activator: ctx.accounts.authority.key(),
        target_balance: runoff.target_balance,
        current_balance: reserve_state.runoff_balance,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
