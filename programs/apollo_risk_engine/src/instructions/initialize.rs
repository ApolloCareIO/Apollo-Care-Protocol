// programs/apollo_risk_engine/src/instructions/initialize.rs

use anchor_lang::prelude::*;
use crate::state::{RiskConfig, RatingTable, CarState, ZoneState, Zone, default_age_bands};
use crate::errors::RiskEngineError;
use crate::events::RiskEngineInitialized;

#[derive(Accounts)]
pub struct InitializeRiskConfig<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + RiskConfig::INIT_SPACE,
        seeds = [RiskConfig::SEED_PREFIX],
        bump
    )]
    pub risk_config: Account<'info, RiskConfig>,

    #[account(
        init,
        payer = authority,
        space = 8 + RatingTable::INIT_SPACE,
        seeds = [RatingTable::SEED_PREFIX],
        bump
    )]
    pub rating_table: Account<'info, RatingTable>,

    #[account(
        init,
        payer = authority,
        space = 8 + CarState::INIT_SPACE,
        seeds = [CarState::SEED_PREFIX],
        bump
    )]
    pub car_state: Account<'info, CarState>,

    #[account(
        init,
        payer = authority,
        space = 8 + ZoneState::INIT_SPACE,
        seeds = [ZoneState::SEED_PREFIX],
        bump
    )]
    pub zone_state: Account<'info, ZoneState>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeRiskConfigParams {
    pub governance_program: Pubkey,
    pub reserves_program: Pubkey,
    pub base_rate_adult: Option<u64>,
    pub initial_expected_annual_claims: u64,
}

pub fn handler(ctx: Context<InitializeRiskConfig>, params: InitializeRiskConfigParams) -> Result<()> {
    let clock = Clock::get()?;

    // Initialize risk config
    let config = &mut ctx.accounts.risk_config;
    config.authority = ctx.accounts.authority.key();
    config.governance_program = params.governance_program;
    config.reserves_program = params.reserves_program;
    config.base_rate_adult = params.base_rate_adult
        .unwrap_or(RiskConfig::DEFAULT_BASE_RATE);
    config.child_factor_bps = RiskConfig::DEFAULT_CHILD_FACTOR_BPS;
    config.max_children = RiskConfig::DEFAULT_MAX_CHILDREN;
    config.tobacco_factor_bps = RiskConfig::DEFAULT_TOBACCO_FACTOR_BPS;
    config.shock_factor_bps = RiskConfig::DEFAULT_SHOCK_FACTOR_BPS;
    config.max_auto_shock_factor_bps = RiskConfig::MAX_AUTO_SHOCK_BPS;
    config.max_committee_shock_factor_bps = RiskConfig::MAX_COMMITTEE_SHOCK_BPS;
    config.max_emergency_shock_factor_bps = RiskConfig::MAX_EMERGENCY_SHOCK_BPS;
    config.min_contribution = 0;
    config.is_active = true;
    config.bump = ctx.bumps.risk_config;
    config.reserved = vec![];

    // Initialize rating table with CMS-compliant defaults
    let rating_table = &mut ctx.accounts.rating_table;
    rating_table.age_bands = default_age_bands();
    rating_table.band_count = rating_table.age_bands.len() as u8;
    rating_table.region_factors = vec![];
    rating_table.last_updated = clock.unix_timestamp;
    rating_table.last_updater = ctx.accounts.authority.key();
    rating_table.bump = ctx.bumps.rating_table;

    // Initialize CAR state
    let car_state = &mut ctx.accounts.car_state;
    car_state.current_car_bps = 0;
    car_state.target_car_bps = CarState::DEFAULT_TARGET_CAR;
    car_state.min_car_bps = CarState::DEFAULT_MIN_CAR;
    car_state.total_usdc_reserves = 0;
    car_state.eligible_aph_usdc = 0;
    car_state.expected_annual_claims = params.initial_expected_annual_claims;
    car_state.current_zone = Zone::Green;
    car_state.last_computed_at = clock.unix_timestamp;
    car_state.bump = ctx.bumps.car_state;

    // Initialize zone state
    let zone_state = &mut ctx.accounts.zone_state;
    zone_state.current_zone = Zone::Green;
    zone_state.green_threshold_bps = ZoneState::DEFAULT_GREEN_BPS;
    zone_state.yellow_threshold_bps = ZoneState::DEFAULT_YELLOW_BPS;
    zone_state.orange_threshold_bps = ZoneState::DEFAULT_ORANGE_BPS;
    zone_state.green_enrollment_cap = ZoneState::GREEN_CAP;
    zone_state.yellow_enrollment_cap = ZoneState::YELLOW_CAP;
    zone_state.orange_enrollment_cap = ZoneState::ORANGE_CAP;
    zone_state.current_month_enrollments = 0;
    zone_state.month_start_timestamp = clock.unix_timestamp;
    zone_state.enrollment_frozen = false;
    zone_state.last_zone_change_at = clock.unix_timestamp;
    zone_state.bump = ctx.bumps.zone_state;

    emit!(RiskEngineInitialized {
        authority: ctx.accounts.authority.key(),
        base_rate_adult: config.base_rate_adult,
        shock_factor_bps: config.shock_factor_bps,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
