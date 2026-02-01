use anchor_lang::prelude::*;

pub mod state;
pub mod errors;
pub mod events;
pub mod instructions;

use instructions::*;
use state::TreatyStatus;

declare_id!("7b2bnKcX2jBZ5VoV9HE7i1HWsFLTUbsLDNLuSjLBsnpo");

/// Apollo Care Protocol - Reinsurance Program
/// 
/// Manages external reinsurance contracts and internal shock absorption mechanisms.
/// 
/// ## Reinsurance Layers
/// 
/// 1. **Specific Stop-Loss**: Per-member threshold ($100k attachment, 80% coverage)
/// 2. **Aggregate Stop-Loss**: Annual total threshold (110% of expected, 100% coverage)
/// 3. **Catastrophic**: Extreme event coverage (150%+ of expected)
/// 4. **Industry Loss Warranty**: Parametric triggers (e.g., pandemic declaration)
/// 
/// ## Key Features
/// 
/// - Treaty management (create, activate, expire)
/// - Recovery claim filing and tracking
/// - Member claims accumulation
/// - Aggregate threshold monitoring
/// - Year-end reconciliation
/// 
/// ## Integration Points
/// 
/// - Claims Program: Records claims to accumulators
/// - Reserves Program: Receives recovery payments
/// - Governance: Treaty approval and modifications
#[program]
pub mod apollo_reinsurance {
    use super::*;
    
    // ========================================================================
    // INITIALIZATION
    // ========================================================================
    
    /// Initialize the global reinsurance configuration
    pub fn initialize_reinsurance(
        ctx: Context<InitializeReinsurance>,
        params: InitializeReinsuranceParams,
    ) -> Result<()> {
        instructions::initialize::handler(ctx, params)
    }
    
    /// Update expected annual claims (mid-year adjustment)
    pub fn update_expected_claims(
        ctx: Context<UpdateReinsuranceConfig>,
        new_expected_claims: u64,
        reason_hash: [u8; 32],
    ) -> Result<()> {
        instructions::initialize::update_expected_claims(ctx, new_expected_claims, reason_hash)
    }
    
    /// Update trigger ratios
    pub fn update_trigger_ratios(
        ctx: Context<UpdateReinsuranceConfig>,
        aggregate_trigger_bps: u16,
        catastrophic_trigger_bps: u16,
        catastrophic_ceiling_bps: u16,
    ) -> Result<()> {
        instructions::initialize::update_trigger_ratios(
            ctx, 
            aggregate_trigger_bps, 
            catastrophic_trigger_bps, 
            catastrophic_ceiling_bps
        )
    }
    
    /// Start a new policy year with fresh counters
    pub fn start_new_policy_year(
        ctx: Context<UpdateReinsuranceConfig>,
        new_year_start: i64,
        new_year_end: i64,
        new_expected_claims: u64,
        new_premium_budget: u64,
    ) -> Result<()> {
        instructions::initialize::start_new_policy_year(
            ctx, 
            new_year_start, 
            new_year_end, 
            new_expected_claims, 
            new_premium_budget
        )
    }
    
    // ========================================================================
    // TREATY MANAGEMENT
    // ========================================================================
    
    /// Create a new reinsurance treaty
    pub fn create_treaty(
        ctx: Context<CreateTreaty>,
        params: CreateTreatyParams,
    ) -> Result<()> {
        instructions::treaties::create_treaty(ctx, params)
    }
    
    /// Activate a pending treaty (requires minimum premium paid)
    pub fn activate_treaty(ctx: Context<ActivateTreaty>) -> Result<()> {
        instructions::treaties::activate_treaty(ctx)
    }
    
    /// Pay reinsurance premium
    pub fn pay_premium(
        ctx: Context<PayPremium>,
        amount: u64,
    ) -> Result<()> {
        instructions::treaties::pay_premium(ctx, amount)
    }
    
    /// Update treaty status (suspend, expire, cancel)
    pub fn update_treaty_status(
        ctx: Context<UpdateTreatyStatus>,
        new_status: TreatyStatus,
        reason_hash: [u8; 32],
    ) -> Result<()> {
        instructions::treaties::update_treaty_status(ctx, new_status, reason_hash)
    }
    
    /// Check and expire treaties past their expiration date (permissionless)
    pub fn check_treaty_expiration(ctx: Context<CheckTreatyExpiration>) -> Result<()> {
        instructions::treaties::check_treaty_expiration(ctx)
    }
    
    /// Update treaty parameters (only while pending)
    pub fn update_treaty_params(
        ctx: Context<UpdateTreatyParams>,
        params: UpdateTreatyParamsInput,
    ) -> Result<()> {
        instructions::treaties::update_treaty_params(ctx, params)
    }
    
    // ========================================================================
    // RECOVERY CLAIMS
    // ========================================================================
    
    /// File a specific stop-loss recovery claim
    pub fn file_specific_recovery(
        ctx: Context<FileSpecificRecovery>,
        params: FileSpecificRecoveryParams,
    ) -> Result<()> {
        instructions::recovery::file_specific_recovery(ctx, params)
    }
    
    /// File an aggregate stop-loss recovery claim
    pub fn file_aggregate_recovery(
        ctx: Context<FileAggregateRecovery>,
        documentation_hash: [u8; 32],
    ) -> Result<()> {
        instructions::recovery::file_aggregate_recovery(ctx, documentation_hash)
    }
    
    /// Submit recovery claim to reinsurer (marks as submitted)
    pub fn submit_recovery_to_reinsurer(
        ctx: Context<SubmitRecoveryToReinsurer>,
        documentation_hash: [u8; 32],
    ) -> Result<()> {
        instructions::recovery::submit_recovery_to_reinsurer(ctx, documentation_hash)
    }
    
    /// Record reinsurer's decision on a claim
    pub fn record_reinsurer_decision(
        ctx: Context<RecordReinsurerDecision>,
        decision: ReinsurerDecision,
    ) -> Result<()> {
        instructions::recovery::record_reinsurer_decision(ctx, decision)
    }
    
    /// Record settlement payment received from reinsurer
    pub fn record_settlement(
        ctx: Context<RecordSettlement>,
        received_amount: u64,
        is_final: bool,
    ) -> Result<()> {
        instructions::recovery::record_settlement(ctx, received_amount, is_final)
    }
    
    // ========================================================================
    // MEMBER ACCUMULATORS
    // ========================================================================
    
    /// Create a member claims accumulator for the policy year
    pub fn create_member_accumulator(
        ctx: Context<CreateMemberAccumulator>,
        member: Pubkey,
        policy_year: u16,
    ) -> Result<()> {
        instructions::accumulator::create_member_accumulator(ctx, member, policy_year)
    }
    
    /// Record a claim to a member's accumulator
    pub fn record_claim_to_accumulator(
        ctx: Context<RecordClaimToAccumulator>,
        claim_amount: u64,
        original_claim_id: u64,
    ) -> Result<()> {
        instructions::accumulator::record_claim_to_accumulator(ctx, claim_amount, original_claim_id)
    }
    
    /// Update accumulator with recovery amount received
    pub fn update_accumulator_recovery(
        ctx: Context<UpdateAccumulatorRecovery>,
        recovered_amount: u64,
    ) -> Result<()> {
        instructions::accumulator::update_accumulator_recovery(ctx, recovered_amount)
    }
    
    // ========================================================================
    // MONTHLY AGGREGATES
    // ========================================================================
    
    /// Initialize monthly aggregate tracking
    pub fn initialize_monthly_aggregate(
        ctx: Context<InitializeMonthlyAggregate>,
        policy_year: u16,
        month: u8,
        expected_claims: u64,
    ) -> Result<()> {
        instructions::accumulator::initialize_monthly_aggregate(ctx, policy_year, month, expected_claims)
    }
    
    /// Update monthly aggregate with claim data
    pub fn update_monthly_aggregate(
        ctx: Context<UpdateMonthlyAggregate>,
        claim_amount: u64,
        is_shock_claim: bool,
        ytd_total: u64,
    ) -> Result<()> {
        instructions::accumulator::update_monthly_aggregate(ctx, claim_amount, is_shock_claim, ytd_total)
    }
    
    // ========================================================================
    // THRESHOLD MONITORING
    // ========================================================================
    
    /// Check aggregate thresholds and trigger if needed (permissionless)
    pub fn check_aggregate_thresholds(ctx: Context<CheckAggregateThresholds>) -> Result<()> {
        instructions::accumulator::check_aggregate_thresholds(ctx)
    }
    
    /// Mark accumulators for year-end reset
    pub fn mark_accumulators_for_reset(ctx: Context<ResetAccumulators>) -> Result<()> {
        instructions::accumulator::mark_accumulators_for_reset(ctx)
    }
}

// ============================================================================
// REINSURANCE CONSTANTS (for reference)
// ============================================================================

/// Default specific stop-loss attachment point ($100,000 USDC)
/// Recommended reduction from docs' $150k for startup phase
pub const DEFAULT_SPECIFIC_ATTACHMENT: u64 = 100_000_000_000; // 100k USDC (6 decimals)

/// Default coinsurance rate for specific stop-loss (20% = 2000 bps)
/// Apollo retains 20%, reinsurer covers 80%
pub const DEFAULT_COINSURANCE_BPS: u16 = 2_000;

/// Default aggregate trigger ratio (110% = 11000 bps)
pub const DEFAULT_AGGREGATE_TRIGGER_BPS: u16 = 11_000;

/// Default catastrophic trigger ratio (150% = 15000 bps)
pub const DEFAULT_CATASTROPHIC_TRIGGER_BPS: u16 = 15_000;

/// Default catastrophic ceiling ratio (300% = 30000 bps)
pub const DEFAULT_CATASTROPHIC_CEILING_BPS: u16 = 30_000;

/// Shock claim threshold ($50,000 USDC)
pub const SHOCK_CLAIM_THRESHOLD: u64 = 50_000_000_000; // 50k USDC
