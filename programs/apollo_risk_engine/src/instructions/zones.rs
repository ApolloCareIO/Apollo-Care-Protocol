// programs/apollo_risk_engine/src/instructions/zones.rs

use anchor_lang::prelude::*;
use crate::state::{RiskConfig, CarState, ZoneState, Zone};
use crate::errors::RiskEngineError;
use crate::events::{ShockFactorUpdated, EnrollmentCapsUpdated, EnrollmentRecorded,
                    EnrollmentFreezeToggled, ZoneThresholdsUpdated};

/// Set ShockFactor (zone-gated)
#[derive(Accounts)]
pub struct SetShockFactor<'info> {
    #[account(
        mut,
        seeds = [RiskConfig::SEED_PREFIX],
        bump = risk_config.bump,
    )]
    pub risk_config: Account<'info, RiskConfig>,

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

    /// Setter must have appropriate authority based on zone
    pub setter: Signer<'info>,
}

pub fn set_shock_factor(ctx: Context<SetShockFactor>, new_shock_factor_bps: u16) -> Result<()> {
    let clock = Clock::get()?;
    let config = &mut ctx.accounts.risk_config;
    let zone = &ctx.accounts.zone_state;

    require!(new_shock_factor_bps >= 10000, RiskEngineError::InvalidBasisPoints); // Min 1.0x

    let old_shock = config.shock_factor_bps;
    let current_zone = zone.current_zone;

    // Determine max allowed ShockFactor based on zone
    let (max_allowed, requires_approval) = match current_zone {
        Zone::Green => (config.max_auto_shock_factor_bps, false),
        Zone::Yellow => (config.max_auto_shock_factor_bps, false),
        Zone::Orange => {
            // Up to 1.5x requires Risk Committee
            if new_shock_factor_bps > config.max_auto_shock_factor_bps {
                (config.max_committee_shock_factor_bps, true)
            } else {
                (config.max_auto_shock_factor_bps, false)
            }
        },
        Zone::Red => {
            // Up to 2.0x requires DAO emergency
            if new_shock_factor_bps > config.max_committee_shock_factor_bps {
                (config.max_emergency_shock_factor_bps, true)
            } else if new_shock_factor_bps > config.max_auto_shock_factor_bps {
                (config.max_committee_shock_factor_bps, true)
            } else {
                (config.max_auto_shock_factor_bps, false)
            }
        }
    };

    require!(
        new_shock_factor_bps <= max_allowed,
        RiskEngineError::ShockFactorExceedsZoneLimit
    );

    // In production, if requires_approval is true, we would validate
    // multisig signatures via CPI to governance program
    // For now, we just check authority
    if requires_approval {
        require!(
            ctx.accounts.setter.key() == config.authority,
            RiskEngineError::ShockFactorRequiresCommittee
        );
    }

    config.shock_factor_bps = new_shock_factor_bps;

    emit!(ShockFactorUpdated {
        old_shock_factor_bps: old_shock,
        new_shock_factor_bps,
        zone: current_zone,
        updater: ctx.accounts.setter.key(),
        requires_approval,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Set enrollment caps for each zone
#[derive(Accounts)]
pub struct SetEnrollmentCaps<'info> {
    #[account(
        seeds = [RiskConfig::SEED_PREFIX],
        bump = risk_config.bump,
    )]
    pub risk_config: Account<'info, RiskConfig>,

    #[account(
        mut,
        seeds = [ZoneState::SEED_PREFIX],
        bump = zone_state.bump,
    )]
    pub zone_state: Account<'info, ZoneState>,

    #[account(
        constraint = authority.key() == risk_config.authority @ RiskEngineError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SetEnrollmentCapsParams {
    pub green_cap: Option<u32>,
    pub yellow_cap: Option<u32>,
    pub orange_cap: Option<u32>,
}

pub fn set_enrollment_caps(ctx: Context<SetEnrollmentCaps>, params: SetEnrollmentCapsParams) -> Result<()> {
    let clock = Clock::get()?;
    let zone_state = &mut ctx.accounts.zone_state;

    if let Some(cap) = params.green_cap {
        zone_state.green_enrollment_cap = cap;
    }
    if let Some(cap) = params.yellow_cap {
        require!(cap > 0, RiskEngineError::InvalidZoneConfig);
        zone_state.yellow_enrollment_cap = cap;
    }
    if let Some(cap) = params.orange_cap {
        require!(cap > 0, RiskEngineError::InvalidZoneConfig);
        zone_state.orange_enrollment_cap = cap;
    }

    emit!(EnrollmentCapsUpdated {
        green_cap: zone_state.green_enrollment_cap,
        yellow_cap: zone_state.yellow_enrollment_cap,
        orange_cap: zone_state.orange_enrollment_cap,
        updater: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Compute zone from current CAR
#[derive(Accounts)]
pub struct ComputeZone<'info> {
    #[account(
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
}

pub fn compute_zone(ctx: Context<ComputeZone>) -> Result<Zone> {
    let car = ctx.accounts.car_state.current_car_bps;
    let zone_state = &mut ctx.accounts.zone_state;
    let clock = Clock::get()?;

    let new_zone = if car >= zone_state.green_threshold_bps {
        Zone::Green
    } else if car >= zone_state.yellow_threshold_bps {
        Zone::Yellow
    } else if car >= zone_state.orange_threshold_bps {
        Zone::Orange
    } else {
        Zone::Red
    };

    // Reset monthly counter if new month
    const MONTH_SECONDS: i64 = 30 * 24 * 60 * 60;
    if clock.unix_timestamp - zone_state.month_start_timestamp > MONTH_SECONDS {
        zone_state.month_start_timestamp = clock.unix_timestamp;
        zone_state.current_month_enrollments = 0;
    }

    zone_state.current_zone = new_zone;

    Ok(new_zone)
}

/// Record an enrollment (called by membership program)
#[derive(Accounts)]
pub struct RecordEnrollment<'info> {
    #[account(
        mut,
        seeds = [ZoneState::SEED_PREFIX],
        bump = zone_state.bump,
    )]
    pub zone_state: Account<'info, ZoneState>,

    /// Member being enrolled
    /// CHECK: Just recording the pubkey
    pub member: UncheckedAccount<'info>,
}

pub fn record_enrollment(ctx: Context<RecordEnrollment>) -> Result<()> {
    let clock = Clock::get()?;
    let zone_state = &mut ctx.accounts.zone_state;

    // Check enrollment is allowed
    require!(!zone_state.enrollment_frozen, RiskEngineError::EnrollmentFrozen);
    require!(zone_state.can_enroll(), RiskEngineError::EnrollmentCapExceeded);

    // Reset counter if new month
    const MONTH_SECONDS: i64 = 30 * 24 * 60 * 60;
    if clock.unix_timestamp - zone_state.month_start_timestamp > MONTH_SECONDS {
        zone_state.month_start_timestamp = clock.unix_timestamp;
        zone_state.current_month_enrollments = 0;
    }

    zone_state.current_month_enrollments += 1;

    emit!(EnrollmentRecorded {
        member: ctx.accounts.member.key(),
        current_zone: zone_state.current_zone,
        month_enrollments: zone_state.current_month_enrollments,
        cap: zone_state.get_current_cap(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Toggle enrollment freeze
#[derive(Accounts)]
pub struct ToggleEnrollmentFreeze<'info> {
    #[account(
        seeds = [RiskConfig::SEED_PREFIX],
        bump = risk_config.bump,
    )]
    pub risk_config: Account<'info, RiskConfig>,

    #[account(
        mut,
        seeds = [ZoneState::SEED_PREFIX],
        bump = zone_state.bump,
    )]
    pub zone_state: Account<'info, ZoneState>,

    #[account(
        constraint = authority.key() == risk_config.authority @ RiskEngineError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn toggle_enrollment_freeze(ctx: Context<ToggleEnrollmentFreeze>, freeze: bool) -> Result<()> {
    let clock = Clock::get()?;
    let zone_state = &mut ctx.accounts.zone_state;

    zone_state.enrollment_frozen = freeze;

    emit!(EnrollmentFreezeToggled {
        frozen: freeze,
        zone: zone_state.current_zone,
        toggler: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Set zone thresholds
#[derive(Accounts)]
pub struct SetZoneThresholds<'info> {
    #[account(
        seeds = [RiskConfig::SEED_PREFIX],
        bump = risk_config.bump,
    )]
    pub risk_config: Account<'info, RiskConfig>,

    #[account(
        mut,
        seeds = [ZoneState::SEED_PREFIX],
        bump = zone_state.bump,
    )]
    pub zone_state: Account<'info, ZoneState>,

    #[account(
        constraint = authority.key() == risk_config.authority @ RiskEngineError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn set_zone_thresholds(
    ctx: Context<SetZoneThresholds>,
    green_bps: Option<u16>,
    yellow_bps: Option<u16>,
    orange_bps: Option<u16>,
) -> Result<()> {
    let clock = Clock::get()?;
    let zone = &mut ctx.accounts.zone_state;

    // Validate ordering: green > yellow > orange
    let new_green = green_bps.unwrap_or(zone.green_threshold_bps);
    let new_yellow = yellow_bps.unwrap_or(zone.yellow_threshold_bps);
    let new_orange = orange_bps.unwrap_or(zone.orange_threshold_bps);

    require!(new_green > new_yellow, RiskEngineError::InvalidZoneConfig);
    require!(new_yellow > new_orange, RiskEngineError::InvalidZoneConfig);
    require!(new_orange > 0, RiskEngineError::InvalidZoneConfig);

    zone.green_threshold_bps = new_green;
    zone.yellow_threshold_bps = new_yellow;
    zone.orange_threshold_bps = new_orange;

    emit!(ZoneThresholdsUpdated {
        green_threshold_bps: new_green,
        yellow_threshold_bps: new_yellow,
        orange_threshold_bps: new_orange,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
