// programs/apollo_reserves/src/lib.rs
//
// Apollo Reserves Program
// =======================
// Manages the three-tier reserve system for Apollo Care:
// - Tier 0: Liquidity Buffer (15-30 days claims)
// - Tier 1: Operating Reserve (60-90 days + IBNR)
// - Tier 2: Contingent Capital (6+ months)
// Plus run-off reserves for wind-down scenarios.

use anchor_lang::prelude::*;

pub mod state;
pub mod errors;
pub mod events;
pub mod instructions;

use instructions::*;

declare_id!("ApR1111111111111111111111111111111111111111");

#[program]
pub mod apollo_reserves {
    use super::*;

    // ==================== INITIALIZATION ====================

    /// Initialize the reserve system
    pub fn initialize_reserves(
        ctx: Context<InitializeReserves>,
        params: InitializeReservesParams,
    ) -> Result<()> {
        instructions::initialize::handler(ctx, params)
    }

    /// Update reserve targets (Actuarial Committee gated)
    pub fn set_reserve_targets(
        ctx: Context<SetReserveTargets>,
        params: SetReserveTargetsParams,
    ) -> Result<()> {
        instructions::initialize::set_reserve_targets(ctx, params)
    }

    // ==================== VAULT MANAGEMENT ====================

    /// Create all USDC vaults for the reserve tiers
    pub fn create_vaults(ctx: Context<CreateVaults>) -> Result<()> {
        instructions::vaults::create_vaults(ctx)
    }

    /// Refill Tier 0 from Tier 1
    pub fn refill_tier0(ctx: Context<RefillTier0>, amount: u64) -> Result<()> {
        instructions::vaults::refill_tier0(ctx, amount)
    }

    /// Refill Tier 1 from Tier 2
    pub fn refill_tier1(ctx: Context<RefillTier1>, amount: u64) -> Result<()> {
        instructions::vaults::refill_tier1(ctx, amount)
    }

    // ==================== CONTRIBUTION ROUTING ====================

    /// Route a member contribution to appropriate vaults
    pub fn route_contribution_to_vaults(
        ctx: Context<RouteContribution>,
        total_amount: u64,
    ) -> Result<()> {
        instructions::routing::route_contribution(ctx, total_amount)
    }

    /// Direct deposit to a specific tier
    pub fn deposit_to_tier(
        ctx: Context<DepositToTier>,
        tier: TierTarget,
        amount: u64,
    ) -> Result<()> {
        instructions::routing::deposit_to_tier(ctx, tier, amount)
    }

    // ==================== IBNR & CLAIMS ESTIMATION ====================

    /// Compute IBNR (Incurred But Not Reported) reserve
    pub fn compute_ibnr(ctx: Context<ComputeIbnr>) -> Result<()> {
        instructions::ibnr::compute_ibnr(ctx)
    }

    /// Update expected claims data
    pub fn update_expected_claims(
        ctx: Context<UpdateExpectedClaims>,
        params: UpdateExpectedClaimsParams,
    ) -> Result<()> {
        instructions::ibnr::update_expected_claims(ctx, params)
    }

    /// Update IBNR parameters directly
    pub fn update_ibnr_params(
        ctx: Context<UpdateIbnrParams>,
        reporting_lag: u16,
        development_factor_bps: u16,
    ) -> Result<()> {
        instructions::ibnr::update_ibnr_params(ctx, reporting_lag, development_factor_bps)
    }

    // ==================== RUN-OFF RESERVE ====================

    /// Fund the run-off reserve
    pub fn fund_runoff_reserve(ctx: Context<FundRunoffReserve>, amount: u64) -> Result<()> {
        instructions::ibnr::fund_runoff_reserve(ctx, amount)
    }

    /// Set run-off parameters
    pub fn set_runoff_params(
        ctx: Context<SetRunoffParams>,
        params: SetRunoffParamsInput,
    ) -> Result<()> {
        instructions::ibnr::set_runoff_params(ctx, params)
    }

    /// Activate run-off mode (protocol wind-down)
    pub fn activate_runoff(ctx: Context<ActivateRunoff>) -> Result<()> {
        instructions::ibnr::activate_runoff(ctx)
    }

    // ==================== CLAIM PAYOUTS ====================

    /// Pay a claim using the waterfall mechanism
    pub fn payout_claim_from_waterfall(
        ctx: Context<PayoutClaimFromWaterfall>,
        params: PayoutParams,
    ) -> Result<()> {
        instructions::payouts::payout_claim_from_waterfall(ctx, params)
    }

    /// Emergency spend from run-off reserve (DAO gated)
    pub fn emergency_spend_runoff(
        ctx: Context<EmergencySpendRunoff>,
        amount: u64,
        reason: String,
    ) -> Result<()> {
        instructions::payouts::emergency_spend_runoff(ctx, amount, reason)
    }

    /// Take a reserve state snapshot (emits event)
    pub fn take_reserve_snapshot(ctx: Context<TakeReserveSnapshot>) -> Result<()> {
        instructions::payouts::take_reserve_snapshot(ctx)
    }
}

/// Public helpers for CPI
pub mod reserve_helpers {
    use super::*;

    pub fn get_reserve_config_seeds() -> &'static [&'static [u8]] {
        &[state::ReserveConfig::SEED_PREFIX]
    }

    pub fn get_vault_authority_seeds() -> &'static [&'static [u8]] {
        &[state::VaultAuthority::SEED_PREFIX]
    }
}
