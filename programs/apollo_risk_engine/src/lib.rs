// programs/apollo_risk_engine/src/lib.rs
//
// Apollo Risk Engine Program
// ==========================
// Handles all actuarial pricing, CAR calculations, and zone management:
// - CMS-compliant age band rating (3:1 max ratio)
// - Regional cost factors
// - Tobacco surcharges
// - Capital Adequacy Ratio (CAR) computation
// - Zone-based enrollment controls (Green/Yellow/Orange/Red)
// - ShockFactor escalation with governance gates

use anchor_lang::prelude::*;

pub mod state;
pub mod errors;
pub mod events;
pub mod instructions;

use instructions::*;
use state::{AgeBand, RegionFactor, ContributionQuote, Zone};

declare_id!("FdTXXgEMT1k5YghXxpe1etxDEBorJ7z1soPmkRQAW8mB");

#[program]
pub mod apollo_risk_engine {
    use super::*;

    // ==================== INITIALIZATION ====================

    /// Initialize the risk engine
    pub fn initialize_risk_config(
        ctx: Context<InitializeRiskConfig>,
        params: InitializeRiskConfigParams,
    ) -> Result<()> {
        instructions::initialize::handler(ctx, params)
    }

    // ==================== RATING TABLE ====================

    /// Set or update the rating table (age bands, regions)
    pub fn set_rating_table(
        ctx: Context<SetRatingTable>,
        params: SetRatingTableParams,
    ) -> Result<()> {
        instructions::rating::set_rating_table(ctx, params)
    }

    /// Quote a contribution for a member
    pub fn quote_contribution(
        ctx: Context<QuoteContribution>,
        params: QuoteContributionParams,
    ) -> Result<ContributionQuote> {
        instructions::rating::quote_contribution(ctx, params)
    }

    /// Update base rate
    pub fn update_base_rate(ctx: Context<UpdateBaseRate>, new_base_rate: u64) -> Result<()> {
        instructions::rating::update_base_rate(ctx, new_base_rate)
    }

    /// Update tobacco factor
    pub fn update_tobacco_factor(ctx: Context<UpdateBaseRate>, new_factor_bps: u16) -> Result<()> {
        instructions::rating::update_tobacco_factor(ctx, new_factor_bps)
    }

    /// Update child factor
    pub fn update_child_factor(ctx: Context<UpdateBaseRate>, new_factor_bps: u16) -> Result<()> {
        instructions::rating::update_child_factor(ctx, new_factor_bps)
    }

    // ==================== CAR MANAGEMENT ====================

    /// Update CAR state with latest reserve and claims data
    pub fn update_car_state(
        ctx: Context<UpdateCarState>,
        params: UpdateCarStateParams,
    ) -> Result<()> {
        instructions::car::update_car_state(ctx, params)
    }

    /// Force recompute CAR (permissionless)
    pub fn recompute_car(ctx: Context<RecomputeCar>) -> Result<u16> {
        instructions::car::recompute_car(ctx)
    }

    /// Get current CAR status
    pub fn get_car_status(ctx: Context<GetCarStatus>) -> Result<CarStatus> {
        instructions::car::get_car_status(ctx)
    }

    /// Set CAR thresholds
    pub fn set_car_thresholds(
        ctx: Context<SetCarThresholds>,
        target_car_bps: Option<u16>,
        min_car_bps: Option<u16>,
    ) -> Result<()> {
        instructions::car::set_car_thresholds(ctx, target_car_bps, min_car_bps)
    }

    // ==================== ZONE MANAGEMENT ====================

    /// Set ShockFactor (zone-gated)
    pub fn set_shock_factor(ctx: Context<SetShockFactor>, new_shock_factor_bps: u16) -> Result<()> {
        instructions::zones::set_shock_factor(ctx, new_shock_factor_bps)
    }

    /// Set enrollment caps per zone
    pub fn set_enrollment_caps(
        ctx: Context<SetEnrollmentCaps>,
        params: SetEnrollmentCapsParams,
    ) -> Result<()> {
        instructions::zones::set_enrollment_caps(ctx, params)
    }

    /// Compute current zone from CAR
    pub fn compute_zone(ctx: Context<ComputeZone>) -> Result<Zone> {
        instructions::zones::compute_zone(ctx)
    }

    /// Record an enrollment (called by membership program)
    pub fn record_enrollment(ctx: Context<RecordEnrollment>) -> Result<()> {
        instructions::zones::record_enrollment(ctx)
    }

    /// Toggle enrollment freeze
    pub fn toggle_enrollment_freeze(ctx: Context<ToggleEnrollmentFreeze>, freeze: bool) -> Result<()> {
        instructions::zones::toggle_enrollment_freeze(ctx, freeze)
    }

    /// Set zone thresholds
    pub fn set_zone_thresholds(
        ctx: Context<SetZoneThresholds>,
        green_bps: Option<u16>,
        yellow_bps: Option<u16>,
        orange_bps: Option<u16>,
    ) -> Result<()> {
        instructions::zones::set_zone_thresholds(ctx, green_bps, yellow_bps, orange_bps)
    }
}

/// Public helpers for CPI
pub mod risk_helpers {
    use super::*;

    pub fn get_risk_config_seeds() -> &'static [&'static [u8]] {
        &[state::RiskConfig::SEED_PREFIX]
    }

    pub fn get_car_state_seeds() -> &'static [&'static [u8]] {
        &[state::CarState::SEED_PREFIX]
    }

    pub fn get_zone_state_seeds() -> &'static [&'static [u8]] {
        &[state::ZoneState::SEED_PREFIX]
    }
}
