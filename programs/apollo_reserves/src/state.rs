// programs/apollo_reserves/src/state.rs

use anchor_lang::prelude::*;
pub use apollo_core::phase::ProtocolPhase;

/// Reserve configuration - defines targets and parameters for the three-tier system
/// PDA seeds: ["reserve_config"]
#[account]
#[derive(InitSpace)]
pub struct ReserveConfig {
    /// DAO authority that can update configuration
    pub authority: Pubkey,

    /// USDC mint address
    pub usdc_mint: Pubkey,

    /// Tier 0 (Liquidity Buffer) target in days of expected claims
    /// Default: 0-30 days
    pub tier0_target_days: u16,

    /// Tier 1 (Operating Reserve) target in days of expected claims
    /// Default: 30-60 days + IBNR
    pub tier1_target_days: u16,

    /// Tier 2 (Contingent Capital) target in days of expected claims
    /// Default: 180+ days (6 months)
    pub tier2_target_days: u16,

    /// Minimum reserve coverage ratio in basis points (10000 = 100%)
    /// Below this triggers emergency actions
    pub min_coverage_ratio_bps: u16,

    /// Target reserve coverage ratio in basis points
    pub target_coverage_ratio_bps: u16,

    /// Percentage of contributions routed to reserves (basis points)
    pub reserve_margin_bps: u16,

    /// Percentage of contributions for admin load (basis points)
    pub admin_load_bps: u16,

    /// Governance program ID for authorization checks
    pub governance_program: Pubkey,

    /// Risk engine program for CAR updates
    pub risk_engine_program: Pubkey,

    /// Is the reserve system initialized and active
    pub is_initialized: bool,

    /// Bump seed
    pub bump: u8,

    /// Reserved for future use
    #[max_len(32)]
    pub reserved: Vec<u8>,
}

impl ReserveConfig {
    pub const SEED_PREFIX: &'static [u8] = b"reserve_config";

    // Default values per Actuarial Specification
    // Tier 0: 0-30 days liquidity buffer
    // Tier 1: 30-60 days (1-2 months) operating reserve
    // Tier 2: 180+ days (6+ months) contingent capital
    pub const DEFAULT_TIER0_DAYS: u16 = 30; // 0-30 days per whitepaper
    pub const DEFAULT_TIER1_DAYS: u16 = 60; // 1-2 months per whitepaper
    pub const DEFAULT_TIER2_DAYS: u16 = 180; // 6+ months per whitepaper

    // MLR = 90%+ REQUIRED → Total loading must be ≤10%
    // Admin (8%) + Reserve (2%) = 10% loading = 90% MLR
    pub const DEFAULT_RESERVE_MARGIN_BPS: u16 = 200; // 2%
    pub const DEFAULT_ADMIN_LOAD_BPS: u16 = 800; // 8%

    pub const DEFAULT_MIN_COVERAGE_BPS: u16 = 10000; // 100%
    pub const DEFAULT_TARGET_COVERAGE_BPS: u16 = 12500; // 125%
}

/// Reserve state - tracks current balances and computed metrics
/// PDA seeds: ["reserve_state"]
#[account]
#[derive(InitSpace)]
pub struct ReserveState {
    /// Current Tier 0 balance (USDC lamports)
    pub tier0_balance: u64,

    /// Current Tier 1 balance (USDC lamports)
    pub tier1_balance: u64,

    /// Current Tier 2 balance (USDC lamports, excluding staked APH)
    pub tier2_balance: u64,

    /// Run-off reserve balance (segregated)
    pub runoff_balance: u64,

    /// Total expected claims (daily average in USDC lamports)
    pub expected_daily_claims: u64,

    /// IBNR (Incurred But Not Reported) reserve amount
    pub ibnr_usdc: u64,

    /// Average reporting lag in days (for IBNR calculation)
    pub avg_reporting_lag_days: u16,

    /// Development factor in basis points (e.g., 11500 = 1.15)
    pub development_factor_bps: u16,

    /// Total claims paid all-time
    pub total_claims_paid: u64,

    /// Total contributions received all-time
    pub total_contributions_received: u64,

    /// Last waterfall execution timestamp
    pub last_waterfall_at: i64,

    /// Last IBNR computation timestamp
    pub last_ibnr_computed_at: i64,

    /// Current computed reserve coverage ratio (bps)
    pub current_coverage_ratio_bps: u16,

    /// Bump seed
    pub bump: u8,
}

impl ReserveState {
    pub const SEED_PREFIX: &'static [u8] = b"reserve_state";
    pub const DEFAULT_DEV_FACTOR_BPS: u16 = 11500; // 1.15
    pub const DEFAULT_REPORTING_LAG: u16 = 21; // 21 days per actuarial spec

    /// Compute total available reserves (Tier0 + Tier1 + Tier2)
    pub fn total_reserves(&self) -> u64 {
        self.tier0_balance
            .saturating_add(self.tier1_balance)
            .saturating_add(self.tier2_balance)
    }

    /// Compute IBNR using formula: avg_daily_claims * reporting_lag * dev_factor
    pub fn compute_ibnr(&self) -> u64 {
        let base = self
            .expected_daily_claims
            .saturating_mul(self.avg_reporting_lag_days as u64);
        // Apply development factor (divide by 10000 for bps)
        base.saturating_mul(self.development_factor_bps as u64) / 10000
    }

    /// Get required Tier 1 reserve based on config
    pub fn required_tier1(&self, config: &ReserveConfig) -> u64 {
        let base_requirement = self
            .expected_daily_claims
            .saturating_mul(config.tier1_target_days as u64);
        base_requirement.saturating_add(self.ibnr_usdc)
    }
}

/// Vault authority PDA - controls all reserve token accounts
/// PDA seeds: ["vault_authority"]
#[account]
#[derive(InitSpace)]
pub struct VaultAuthority {
    /// Tier 0 vault (liquidity buffer) token account
    pub tier0_vault: Pubkey,

    /// Tier 1 vault (operating reserve) token account
    pub tier1_vault: Pubkey,

    /// Tier 2 vault (contingent capital) token account
    pub tier2_vault: Pubkey,

    /// Run-off reserve vault token account
    pub runoff_vault: Pubkey,

    /// Admin/operations vault token account
    pub admin_vault: Pubkey,

    /// USDC mint this authority controls
    pub usdc_mint: Pubkey,

    /// Nonce for PDA signing
    pub bump: u8,
}

impl VaultAuthority {
    pub const SEED_PREFIX: &'static [u8] = b"vault_authority";
}

/// Run-off state for wind-down scenarios
/// PDA seeds: ["runoff_state"]
#[account]
#[derive(InitSpace)]
pub struct RunoffState {
    /// Target run-off balance (180 days IBNR + admin + legal)
    pub target_balance: u64,

    /// Estimated legal costs for wind-down
    pub estimated_legal_costs: u64,

    /// Estimated admin costs for wind-down (per month)
    pub monthly_admin_costs: u64,

    /// Wind-down duration in months
    pub winddown_months: u8,

    /// Is run-off mode active (protocol winding down)
    pub runoff_active: bool,

    /// Run-off activated timestamp
    pub runoff_activated_at: i64,

    /// Bump seed
    pub bump: u8,
}

impl RunoffState {
    pub const SEED_PREFIX: &'static [u8] = b"runoff_state";
    pub const DEFAULT_WINDDOWN_MONTHS: u8 = 6;

    /// Calculate total required run-off reserve
    pub fn required_runoff_reserve(&self, ibnr: u64) -> u64 {
        let admin_total =
            (self.monthly_admin_costs as u64).saturating_mul(self.winddown_months as u64);
        ibnr.saturating_add(admin_total)
            .saturating_add(self.estimated_legal_costs)
    }
}

/// IBNR parameters for actuarial calculation
/// PDA seeds: ["ibnr_params"]
#[account]
#[derive(InitSpace)]
pub struct IbnrParams {
    /// Average daily claims (rolling 30-day)
    pub avg_daily_claims_30d: u64,

    /// Average daily claims (rolling 90-day)
    pub avg_daily_claims_90d: u64,

    /// Average reporting lag observed (days)
    pub observed_reporting_lag: u16,

    /// Development factor (bps) - how much claims grow after initial report
    pub development_factor_bps: u16,

    /// Standard deviation of daily claims (for volatility)
    pub claims_std_dev: u64,

    /// Last updated timestamp
    pub last_updated: i64,

    /// Number of claims in sample
    pub sample_size: u32,

    /// Bump seed
    pub bump: u8,
}

impl IbnrParams {
    pub const SEED_PREFIX: &'static [u8] = b"ibnr_params";
}

/// Waterfall execution log - tracks payout sources
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum WaterfallSource {
    Tier0,
    Tier1,
    Tier2,
    Runoff,
    Staked,
}

/// Contribution routing record
#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct ContributionRouting {
    /// Amount to Tier 0 (liquidity)
    pub to_tier0: u64,
    /// Amount to Tier 1 (reserve margin)
    pub to_tier1: u64,
    /// Amount to Tier 2 (treasury)
    pub to_tier2: u64,
    /// Amount to admin/operations
    pub to_admin: u64,
    /// Total contribution
    pub total: u64,
}

// =============================================================================
// REINSURANCE CONFIGURATION
// Critical at bootstrap scale for variance smoothing
// =============================================================================

/// Reinsurance policy configuration
/// PDA seeds: ["reinsurance_config"]
#[account]
#[derive(InitSpace)]
pub struct ReinsuranceConfig {
    /// Authority for updates
    pub authority: Pubkey,

    /// Specific stop-loss attachment point (USDC)
    /// Claims above this are partially covered by reinsurer
    pub specific_attachment: u64,

    /// Specific stop-loss coverage percentage (bps, e.g., 9000 = 90%)
    pub specific_coverage_bps: u16,

    /// Aggregate stop-loss trigger (bps of expected annual claims)
    /// When total claims exceed this %, aggregate coverage kicks in
    pub aggregate_trigger_bps: u16,

    /// Aggregate stop-loss ceiling (bps of expected annual claims)
    pub aggregate_ceiling_bps: u16,

    /// Aggregate coverage percentage (bps)
    pub aggregate_coverage_bps: u16,

    /// Current policy period start timestamp
    pub policy_period_start: i64,

    /// Current policy period end timestamp
    pub policy_period_end: i64,

    /// Expected annual claims for policy period (USDC)
    pub expected_annual_claims: u64,

    /// Claims paid under specific stop-loss this period
    pub specific_claims_paid: u64,

    /// Claims recovered from reinsurer (specific)
    pub specific_recovered: u64,

    /// Claims applied to aggregate this period
    pub aggregate_claims_accumulated: u64,

    /// Claims recovered from reinsurer (aggregate)
    pub aggregate_recovered: u64,

    /// Total reinsurance premium paid for current policy
    pub reinsurance_premium_paid: u64,

    /// Is reinsurance active
    pub is_active: bool,

    /// Bump seed
    pub bump: u8,
}

impl ReinsuranceConfig {
    pub const SEED_PREFIX: &'static [u8] = b"reinsurance_config";

    // Bootstrap defaults - more protective for small pools
    pub const BOOTSTRAP_SPECIFIC_ATTACHMENT: u64 = 50_000_000_000; // $50k
    pub const BOOTSTRAP_SPECIFIC_COVERAGE_BPS: u16 = 9000; // 90%
    pub const BOOTSTRAP_AGGREGATE_TRIGGER_BPS: u16 = 11000; // 110%
    pub const BOOTSTRAP_AGGREGATE_CEILING_BPS: u16 = 15000; // 150%
    pub const BOOTSTRAP_AGGREGATE_COVERAGE_BPS: u16 = 10000; // 100%

    // Standard defaults - higher attachment for larger pools
    pub const STANDARD_SPECIFIC_ATTACHMENT: u64 = 100_000_000_000; // $100k
    pub const STANDARD_AGGREGATE_TRIGGER_BPS: u16 = 11000; // 110%
    pub const STANDARD_AGGREGATE_CEILING_BPS: u16 = 15000; // 150%

    /// Check if a claim triggers specific stop-loss
    pub fn triggers_specific(&self, claim_amount: u64) -> bool {
        claim_amount > self.specific_attachment
    }

    /// Calculate specific stop-loss recovery amount
    pub fn calculate_specific_recovery(&self, claim_amount: u64) -> u64 {
        if claim_amount <= self.specific_attachment {
            return 0;
        }

        let excess = claim_amount - self.specific_attachment;
        (excess as u128 * self.specific_coverage_bps as u128 / 10000) as u64
    }

    /// Check if aggregate stop-loss is triggered
    pub fn triggers_aggregate(&self) -> bool {
        if self.expected_annual_claims == 0 {
            return false;
        }

        let ratio = self
            .aggregate_claims_accumulated
            .saturating_mul(10000)
            .checked_div(self.expected_annual_claims)
            .unwrap_or(0);

        ratio >= self.aggregate_trigger_bps as u64
    }
}

// =============================================================================
// PHASE MANAGER - Tracks protocol evolution from HCSM to Licensed Insurer
// =============================================================================

/// Phase manager - controls protocol phase transitions
/// PDA seeds: ["phase_manager"]
#[account]
#[derive(InitSpace)]
pub struct PhaseManager {
    /// Authority for phase transitions
    pub authority: Pubkey,

    /// Current operational phase
    pub current_phase: ProtocolPhase,

    /// Phase 1 start timestamp
    pub phase1_start: i64,

    /// Phase 2 start timestamp (0 if not reached)
    pub phase2_start: i64,

    /// Phase 3 start timestamp (0 if not reached)
    pub phase3_start: i64,

    /// Phase 1 → 2 requirements
    pub phase1_requirements: Phase1Requirements,

    /// Phase 2 → 3 requirements
    pub phase2_requirements: Phase2Requirements,

    /// Is transition pending DAO vote
    pub transition_pending: bool,

    /// Pending transition target phase
    pub pending_target_phase: ProtocolPhase,

    /// Bump seed
    pub bump: u8,
}

impl PhaseManager {
    pub const SEED_PREFIX: &'static [u8] = b"phase_manager";
}

// ProtocolPhase imported from apollo_core::phase (single source of truth)

/// Requirements for Phase 1 → Phase 2 transition
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct Phase1Requirements {
    /// Minimum months of successful operation
    pub min_months_operation: u8,
    /// Minimum active members
    pub min_members: u32,
    /// Minimum loss ratio (bps) - ensures adequate pricing
    pub min_loss_ratio_bps: u16,
    /// Maximum loss ratio (bps) - ensures solvency
    pub max_loss_ratio_bps: u16,
    /// Consecutive months within loss ratio bounds
    pub consecutive_good_months: u8,
    /// Minimum CAR maintained throughout
    pub min_car_bps: u16,
    /// Smart contract audit completed
    pub audit_completed: bool,
    /// Financial audit completed
    pub financial_audit_completed: bool,
}

impl Default for Phase1Requirements {
    fn default() -> Self {
        Self {
            min_months_operation: 12,
            min_members: 300,
            min_loss_ratio_bps: 8500, // 85% minimum (proves revenue)
            max_loss_ratio_bps: 9500, // 95% maximum (proves solvency)
            consecutive_good_months: 6,
            min_car_bps: 12500, // 125%
            audit_completed: false,
            financial_audit_completed: false,
        }
    }
}

/// Requirements for Phase 2 → Phase 3 transition
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct Phase2Requirements {
    /// Minimum months in sandbox
    pub min_months_sandbox: u8,
    /// Minimum active members
    pub min_members: u32,
    /// Regulatory approval received
    pub regulatory_approval: bool,
    /// Statutory capital requirements met
    pub statutory_capital_met: bool,
    /// Actuarial certification obtained
    pub actuarial_certification: bool,
    /// All required committees established
    pub committees_established: bool,
}

impl Default for Phase2Requirements {
    fn default() -> Self {
        Self {
            min_months_sandbox: 12,
            min_members: 1000,
            regulatory_approval: false,
            statutory_capital_met: false,
            actuarial_certification: false,
            committees_established: false,
        }
    }
}

// =============================================================================
// COHORT METRICS - Track adverse selection by enrollment cohort
// =============================================================================

/// Cohort metrics for adverse selection monitoring
/// PDA seeds: ["cohort", cohort_id]
#[account]
#[derive(InitSpace)]
pub struct CohortMetrics {
    /// Cohort identifier (typically enrollment month: YYYYMM)
    pub cohort_id: u32,

    /// Number of members in this cohort
    pub member_count: u32,

    /// Members still active
    pub active_count: u32,

    /// Total premiums collected from cohort (USDC)
    pub total_premiums: u64,

    /// Total claims paid for cohort (USDC)
    pub total_claims: u64,

    /// Cohort loss ratio (bps)
    pub loss_ratio_bps: u16,

    /// Months since cohort formation
    pub months_active: u8,

    /// Is flagged for review (high loss ratio)
    pub flagged: bool,

    /// Bump seed
    pub bump: u8,
}

impl CohortMetrics {
    pub const SEED_PREFIX: &'static [u8] = b"cohort";

    /// Loss ratio threshold for flagging (120%)
    pub const LOSS_RATIO_ALERT_BPS: u16 = 12000;

    /// Update loss ratio calculation
    pub fn update_loss_ratio(&mut self) {
        if self.total_premiums == 0 {
            self.loss_ratio_bps = 0;
            return;
        }

        let ratio = self
            .total_claims
            .saturating_mul(10000)
            .checked_div(self.total_premiums)
            .unwrap_or(0);

        self.loss_ratio_bps = ratio.min(u16::MAX as u64) as u16;
        self.flagged = self.loss_ratio_bps > Self::LOSS_RATIO_ALERT_BPS;
    }
}

// ==================== UNIT TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== RESERVE STATE TESTS ====================

    fn create_test_reserve_state() -> ReserveState {
        ReserveState {
            tier0_balance: 1_000_000_000_000,                 // $1M
            tier1_balance: 5_000_000_000_000,                 // $5M
            tier2_balance: 10_000_000_000_000,                // $10M
            runoff_balance: 2_000_000_000_000,                // $2M
            expected_daily_claims: 100_000_000_000,           // $100k/day
            ibnr_usdc: 2_415_000_000_000,                     // ~$2.415M ($100k * 21 * 1.15)
            avg_reporting_lag_days: 21,                       // 21 days per actuarial spec
            development_factor_bps: 11500,                    // 1.15
            total_claims_paid: 50_000_000_000_000,            // $50M
            total_contributions_received: 60_000_000_000_000, // $60M
            last_waterfall_at: 0,
            last_ibnr_computed_at: 0,
            current_coverage_ratio_bps: 16000, // 160%
            bump: 255,
        }
    }

    #[test]
    fn test_total_reserves() {
        let state = create_test_reserve_state();
        let total = state.total_reserves();

        // $1M + $5M + $10M = $16M
        assert_eq!(total, 16_000_000_000_000);
    }

    #[test]
    fn test_total_reserves_overflow_protection() {
        let mut state = create_test_reserve_state();
        state.tier0_balance = u64::MAX;
        state.tier1_balance = u64::MAX;
        state.tier2_balance = u64::MAX;

        // Should not panic, should saturate
        let total = state.total_reserves();
        assert_eq!(total, u64::MAX);
    }

    #[test]
    fn test_compute_ibnr() {
        let state = create_test_reserve_state();
        let ibnr = state.compute_ibnr();

        // $100k/day * 21 days * 1.15 = $2,415,000
        // = 100_000_000_000 * 21 * 11500 / 10000
        let expected = 100_000_000_000u64 * 21 * 11500 / 10000;
        assert_eq!(ibnr, expected);
    }

    #[test]
    fn test_compute_ibnr_zero_claims() {
        let mut state = create_test_reserve_state();
        state.expected_daily_claims = 0;

        let ibnr = state.compute_ibnr();
        assert_eq!(ibnr, 0);
    }

    #[test]
    fn test_compute_ibnr_zero_lag() {
        let mut state = create_test_reserve_state();
        state.avg_reporting_lag_days = 0;

        let ibnr = state.compute_ibnr();
        assert_eq!(ibnr, 0);
    }

    #[test]
    fn test_required_tier1() {
        let state = create_test_reserve_state();
        let config = ReserveConfig {
            authority: Pubkey::default(),
            usdc_mint: Pubkey::default(),
            tier0_target_days: 15,
            tier1_target_days: 60,
            tier2_target_days: 180,
            min_coverage_ratio_bps: 10000,
            target_coverage_ratio_bps: 12500,
            // MLR = 90%+ requires 10% max loading: 8% admin + 2% reserve
            reserve_margin_bps: 200,
            admin_load_bps: 800,
            governance_program: Pubkey::default(),
            risk_engine_program: Pubkey::default(),
            is_initialized: true,
            bump: 255,
            reserved: vec![],
        };

        let required = state.required_tier1(&config);

        // $100k/day * 60 days + IBNR
        // = 6_000_000_000_000 + 1_610_000_000_000 = 7_610_000_000_000
        let expected = 100_000_000_000u64 * 60 + state.ibnr_usdc;
        assert_eq!(required, expected);
    }

    // ==================== RUNOFF STATE TESTS ====================

    fn create_test_runoff_state() -> RunoffState {
        RunoffState {
            target_balance: 5_000_000_000_000,      // $5M
            estimated_legal_costs: 500_000_000_000, // $500k
            monthly_admin_costs: 100_000_000_000,   // $100k/month
            winddown_months: 6,
            runoff_active: false,
            runoff_activated_at: 0,
            bump: 255,
        }
    }

    #[test]
    fn test_required_runoff_reserve() {
        let state = create_test_runoff_state();
        let ibnr = 1_610_000_000_000u64; // $1.61M

        let required = state.required_runoff_reserve(ibnr);

        // IBNR + (6 months * $100k) + $500k legal
        // = $1.61M + $600k + $500k = $2.71M
        let expected = ibnr + (100_000_000_000u64 * 6) + 500_000_000_000;
        assert_eq!(required, expected);
    }

    #[test]
    fn test_required_runoff_reserve_zero_ibnr() {
        let state = create_test_runoff_state();
        let ibnr = 0u64;

        let required = state.required_runoff_reserve(ibnr);

        // 0 + (6 months * $100k) + $500k = $1.1M
        let expected = (100_000_000_000u64 * 6) + 500_000_000_000;
        assert_eq!(required, expected);
    }

    #[test]
    fn test_required_runoff_reserve_zero_months() {
        let mut state = create_test_runoff_state();
        state.winddown_months = 0;
        let ibnr = 1_000_000_000_000u64;

        let required = state.required_runoff_reserve(ibnr);

        // IBNR + 0 + $500k
        let expected = ibnr + 500_000_000_000;
        assert_eq!(required, expected);
    }

    // ==================== RESERVE CONFIG TESTS ====================

    #[test]
    fn test_reserve_config_defaults() {
        assert_eq!(ReserveConfig::DEFAULT_TIER0_DAYS, 30); // 0-30 days
        assert_eq!(ReserveConfig::DEFAULT_TIER1_DAYS, 60); // 30-60 days (1-2 months)
        assert_eq!(ReserveConfig::DEFAULT_TIER2_DAYS, 180); // 6+ months
                                                            // MLR = 90%+ requires total loading ≤10%
                                                            // Admin (8%) + Reserve (2%) = 10% loading
        assert_eq!(ReserveConfig::DEFAULT_RESERVE_MARGIN_BPS, 200); // 2%
        assert_eq!(ReserveConfig::DEFAULT_ADMIN_LOAD_BPS, 800); // 8%
        assert_eq!(ReserveConfig::DEFAULT_MIN_COVERAGE_BPS, 10000); // 100%
        assert_eq!(ReserveConfig::DEFAULT_TARGET_COVERAGE_BPS, 12500); // 125%
    }

    #[test]
    fn test_reserve_state_defaults() {
        assert_eq!(ReserveState::DEFAULT_DEV_FACTOR_BPS, 11500); // 1.15
        assert_eq!(ReserveState::DEFAULT_REPORTING_LAG, 21);
    }

    #[test]
    fn test_runoff_state_defaults() {
        assert_eq!(RunoffState::DEFAULT_WINDDOWN_MONTHS, 6);
    }

    // ==================== SEED PREFIX TESTS ====================

    #[test]
    fn test_seed_prefixes() {
        assert_eq!(ReserveConfig::SEED_PREFIX, b"reserve_config");
        assert_eq!(ReserveState::SEED_PREFIX, b"reserve_state");
        assert_eq!(VaultAuthority::SEED_PREFIX, b"vault_authority");
        assert_eq!(RunoffState::SEED_PREFIX, b"runoff_state");
        assert_eq!(IbnrParams::SEED_PREFIX, b"ibnr_params");
    }

    // ==================== WATERFALL SOURCE TESTS ====================

    #[test]
    fn test_waterfall_source_equality() {
        assert_eq!(WaterfallSource::Tier0, WaterfallSource::Tier0);
        assert_ne!(WaterfallSource::Tier0, WaterfallSource::Tier1);
        assert_ne!(WaterfallSource::Tier1, WaterfallSource::Tier2);
    }

    // ==================== CONTRIBUTION ROUTING TESTS ====================

    #[test]
    fn test_contribution_routing_struct() {
        let routing = ContributionRouting {
            to_tier0: 100_000_000, // $0.10
            to_tier1: 200_000_000, // $0.20
            to_tier2: 500_000_000, // $0.50
            to_admin: 200_000_000, // $0.20
            total: 1_000_000_000,  // $1.00
        };

        // Verify sum equals total
        let sum = routing.to_tier0 + routing.to_tier1 + routing.to_tier2 + routing.to_admin;
        assert_eq!(sum, routing.total);
    }

    // ==================== EDGE CASE TESTS ====================

    #[test]
    fn test_large_values() {
        let mut state = create_test_reserve_state();
        state.tier0_balance = 1_000_000_000_000_000; // $1 quadrillion (extreme case)
        state.tier1_balance = 1_000_000_000_000_000;
        state.tier2_balance = 1_000_000_000_000_000;

        let total = state.total_reserves();
        assert_eq!(total, 3_000_000_000_000_000);
    }

    #[test]
    fn test_zero_values() {
        let state = ReserveState {
            tier0_balance: 0,
            tier1_balance: 0,
            tier2_balance: 0,
            runoff_balance: 0,
            expected_daily_claims: 0,
            ibnr_usdc: 0,
            avg_reporting_lag_days: 0,
            development_factor_bps: 0,
            total_claims_paid: 0,
            total_contributions_received: 0,
            last_waterfall_at: 0,
            last_ibnr_computed_at: 0,
            current_coverage_ratio_bps: 0,
            bump: 0,
        };

        assert_eq!(state.total_reserves(), 0);
        assert_eq!(state.compute_ibnr(), 0);
    }
}
