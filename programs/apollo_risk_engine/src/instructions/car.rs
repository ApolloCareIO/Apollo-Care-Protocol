// programs/apollo_risk_engine/src/instructions/car.rs

use crate::errors::RiskEngineError;
use crate::events::{CarStateUpdated, ZoneTransition};
use crate::state::{CarState, RiskConfig, Zone, ZoneState};
use anchor_lang::prelude::*;

/// Update CAR state with latest data
#[derive(Accounts)]
pub struct UpdateCarState<'info> {
    #[account(
        seeds = [RiskConfig::SEED_PREFIX],
        bump = risk_config.bump,
    )]
    pub risk_config: Account<'info, RiskConfig>,

    #[account(
        mut,
        seeds = [CarState::SEED_PREFIX],
        bump = car_state.bump,
    )]
    pub car_state: Account<'info, CarState>,

    #[account(
        mut,
        seeds = [ZoneState::SEED_PREFIX],
        bump = zone_state.bump,
    )]
    pub zone_state: Account<'info, ZoneState>,

    /// Updater - can be reserves program, staking program, or authority
    pub updater: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateCarStateParams {
    /// Total USDC in reserve tiers
    pub total_usdc_reserves: Option<u64>,
    /// Eligible staked APH value (after haircut)
    pub eligible_aph_usdc: Option<u64>,
    /// Expected annual claims
    pub expected_annual_claims: Option<u64>,
}

pub fn update_car_state(ctx: Context<UpdateCarState>, params: UpdateCarStateParams) -> Result<()> {
    let clock = Clock::get()?;
    let car_state = &mut ctx.accounts.car_state;
    let zone_state = &mut ctx.accounts.zone_state;

    let old_car = car_state.current_car_bps;
    let old_zone = car_state.current_zone;

    // Update inputs if provided
    if let Some(usdc) = params.total_usdc_reserves {
        car_state.total_usdc_reserves = usdc;
    }
    if let Some(aph) = params.eligible_aph_usdc {
        car_state.eligible_aph_usdc = aph;
    }
    if let Some(claims) = params.expected_annual_claims {
        require!(claims > 0, RiskEngineError::ZeroExpectedClaims);
        car_state.expected_annual_claims = claims;
    }

    // Recompute CAR
    let new_car = car_state.compute_car();
    car_state.current_car_bps = new_car;
    car_state.last_computed_at = clock.unix_timestamp;

    // Determine new zone
    let new_zone = determine_zone(new_car, zone_state);
    car_state.current_zone = new_zone;

    // Update zone state if changed
    if new_zone != old_zone {
        zone_state.current_zone = new_zone;
        zone_state.last_zone_change_at = clock.unix_timestamp;

        // Handle zone-specific actions
        match new_zone {
            Zone::Red => {
                zone_state.enrollment_frozen = true;
            }
            Zone::Orange | Zone::Yellow | Zone::Green => {
                // Enrollment may be unfrozen if coming from Red
                if old_zone == Zone::Red {
                    zone_state.enrollment_frozen = false;
                }
            }
        }

        emit!(ZoneTransition {
            old_zone,
            new_zone,
            car_bps: new_car,
            timestamp: clock.unix_timestamp,
        });
    }

    emit!(CarStateUpdated {
        old_car_bps: old_car,
        new_car_bps: new_car,
        total_usdc_reserves: car_state.total_usdc_reserves,
        eligible_aph_usdc: car_state.eligible_aph_usdc,
        expected_annual_claims: car_state.expected_annual_claims,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Determine zone based on CAR and zone thresholds
fn determine_zone(car_bps: u16, zone_state: &ZoneState) -> Zone {
    if car_bps >= zone_state.green_threshold_bps {
        Zone::Green
    } else if car_bps >= zone_state.yellow_threshold_bps {
        Zone::Yellow
    } else if car_bps >= zone_state.orange_threshold_bps {
        Zone::Orange
    } else {
        Zone::Red
    }
}

/// Force recompute CAR (anyone can call)
#[derive(Accounts)]
pub struct RecomputeCar<'info> {
    #[account(
        seeds = [CarState::SEED_PREFIX],
        bump = car_state.bump,
    )]
    pub car_state: Account<'info, CarState>,

    #[account(
        seeds = [ZoneState::SEED_PREFIX],
        bump = zone_state.bump,
    )]
    pub zone_state: Account<'info, ZoneState>,
}

pub fn recompute_car(ctx: Context<RecomputeCar>) -> Result<u16> {
    let car_state = &ctx.accounts.car_state;
    Ok(car_state.compute_car())
}

/// Get current CAR and zone (view function)
#[derive(Accounts)]
pub struct GetCarStatus<'info> {
    #[account(
        seeds = [CarState::SEED_PREFIX],
        bump = car_state.bump,
    )]
    pub car_state: Account<'info, CarState>,

    #[account(
        seeds = [ZoneState::SEED_PREFIX],
        bump = zone_state.bump,
    )]
    pub zone_state: Account<'info, ZoneState>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CarStatus {
    pub car_bps: u16,
    pub target_car_bps: u16,
    pub min_car_bps: u16,
    pub current_zone: Zone,
    pub total_usdc_reserves: u64,
    pub eligible_aph_usdc: u64,
    pub expected_annual_claims: u64,
    pub enrollment_frozen: bool,
    pub enrollment_cap: u32,
    pub current_month_enrollments: u32,
}

pub fn get_car_status(ctx: Context<GetCarStatus>) -> Result<CarStatus> {
    let car = &ctx.accounts.car_state;
    let zone = &ctx.accounts.zone_state;

    Ok(CarStatus {
        car_bps: car.current_car_bps,
        target_car_bps: car.target_car_bps,
        min_car_bps: car.min_car_bps,
        current_zone: zone.current_zone,
        total_usdc_reserves: car.total_usdc_reserves,
        eligible_aph_usdc: car.eligible_aph_usdc,
        expected_annual_claims: car.expected_annual_claims,
        enrollment_frozen: zone.enrollment_frozen,
        enrollment_cap: zone.get_current_cap(),
        current_month_enrollments: zone.current_month_enrollments,
    })
}

/// Set CAR thresholds (requires DAO authority)
#[derive(Accounts)]
pub struct SetCarThresholds<'info> {
    #[account(
        seeds = [RiskConfig::SEED_PREFIX],
        bump = risk_config.bump,
    )]
    pub risk_config: Account<'info, RiskConfig>,

    #[account(
        mut,
        seeds = [CarState::SEED_PREFIX],
        bump = car_state.bump,
    )]
    pub car_state: Account<'info, CarState>,

    #[account(
        constraint = authority.key() == risk_config.authority @ RiskEngineError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn set_car_thresholds(
    ctx: Context<SetCarThresholds>,
    target_car_bps: Option<u16>,
    min_car_bps: Option<u16>,
) -> Result<()> {
    let car_state = &mut ctx.accounts.car_state;

    if let Some(target) = target_car_bps {
        require!(target >= 10000, RiskEngineError::InvalidBasisPoints); // At least 100%
        car_state.target_car_bps = target;
    }

    if let Some(min) = min_car_bps {
        require!(
            min >= 5000 && min < car_state.target_car_bps,
            RiskEngineError::InvalidBasisPoints
        );
        car_state.min_car_bps = min;
    }

    Ok(())
}
