// programs/apollo_risk_engine/src/events.rs

use anchor_lang::prelude::*;
use crate::state::Zone;

/// Emitted when risk engine is initialized
#[event]
pub struct RiskEngineInitialized {
    pub authority: Pubkey,
    pub base_rate_adult: u64,
    pub shock_factor_bps: u16,
    pub timestamp: i64,
}

/// Emitted when rating table is updated
#[event]
pub struct RatingTableUpdated {
    pub updater: Pubkey,
    pub band_count: u8,
    pub region_count: u8,
    pub timestamp: i64,
}

/// Emitted when a contribution is quoted
#[event]
pub struct ContributionQuoted {
    pub member: Pubkey,
    pub age: u8,
    pub is_tobacco: bool,
    pub region_code: u8,
    pub base_amount: u64,
    pub final_contribution: u64,
    pub timestamp: i64,
}

/// Emitted when CAR state is updated
#[event]
pub struct CarStateUpdated {
    pub old_car_bps: u16,
    pub new_car_bps: u16,
    pub total_usdc_reserves: u64,
    pub eligible_aph_usdc: u64,
    pub expected_annual_claims: u64,
    pub timestamp: i64,
}

/// Emitted when zone changes
#[event]
pub struct ZoneTransition {
    pub old_zone: Zone,
    pub new_zone: Zone,
    pub car_bps: u16,
    pub timestamp: i64,
}

/// Emitted when ShockFactor is updated
#[event]
pub struct ShockFactorUpdated {
    pub old_shock_factor_bps: u16,
    pub new_shock_factor_bps: u16,
    pub zone: Zone,
    pub updater: Pubkey,
    pub requires_approval: bool,
    pub timestamp: i64,
}

/// Emitted when enrollment caps are set
#[event]
pub struct EnrollmentCapsUpdated {
    pub green_cap: u32,
    pub yellow_cap: u32,
    pub orange_cap: u32,
    pub updater: Pubkey,
    pub timestamp: i64,
}

/// Emitted when enrollment is counted
#[event]
pub struct EnrollmentRecorded {
    pub member: Pubkey,
    pub current_zone: Zone,
    pub month_enrollments: u32,
    pub cap: u32,
    pub timestamp: i64,
}

/// Emitted when enrollment freeze is toggled
#[event]
pub struct EnrollmentFreezeToggled {
    pub frozen: bool,
    pub zone: Zone,
    pub toggler: Pubkey,
    pub timestamp: i64,
}

/// Emitted when zone thresholds are updated
#[event]
pub struct ZoneThresholdsUpdated {
    pub green_threshold_bps: u16,
    pub yellow_threshold_bps: u16,
    pub orange_threshold_bps: u16,
    pub timestamp: i64,
}
