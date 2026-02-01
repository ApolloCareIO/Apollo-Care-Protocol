use anchor_lang::prelude::*;

/// ============================================================================
/// REINSURANCE LAYER TYPES
/// ============================================================================

/// Type of reinsurance layer
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReinsuranceLayerType {
    /// Per-individual stop-loss coverage
    /// Triggers when a single member's claims exceed attachment point
    SpecificStopLoss,
    
    /// Annual aggregate stop-loss coverage  
    /// Triggers when total annual claims exceed expected claims threshold
    AggregateStopLoss,
    
    /// Catastrophic coverage for extreme events
    /// Triggers at higher threshold than aggregate (e.g., 150%+ of expected)
    Catastrophic,
    
    /// Industry Loss Warranty - parametric trigger based on external events
    /// E.g., pandemic declarations, natural disaster declarations
    IndustryLossWarranty,
}

/// Status of a reinsurance treaty
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TreatyStatus {
    /// Treaty is being negotiated/set up
    #[default]
    Pending,
    
    /// Treaty is active and can receive claims
    Active,
    
    /// Treaty is suspended (e.g., premium not paid)
    Suspended,
    
    /// Treaty has expired
    Expired,
    
    /// Treaty was cancelled
    Cancelled,
}

/// Status of a recovery claim
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RecoveryStatus {
    /// Claim is being calculated/prepared
    #[default]
    Pending,
    
    /// Claim has been submitted to reinsurer
    Submitted,
    
    /// Claim is under review by reinsurer
    UnderReview,
    
    /// Claim has been approved
    Approved,
    
    /// Payment has been received
    Settled,
    
    /// Claim was partially paid
    PartiallySettled,
    
    /// Claim was disputed by reinsurer
    Disputed,
    
    /// Claim was denied
    Denied,
}

/// ============================================================================
/// REINSURANCE GLOBAL CONFIG
/// ============================================================================

/// Global reinsurance configuration for the protocol
#[account]
#[derive(Default)]
pub struct ReinsuranceConfig {
    /// Program authority (multisig/DAO)
    pub authority: Pubkey,
    
    /// Reinsurance committee multisig (subset of governance)
    pub reinsurance_committee: Pubkey,
    
    /// Current policy year start timestamp
    pub policy_year_start: i64,
    
    /// Current policy year end timestamp
    pub policy_year_end: i64,
    
    /// Expected annual claims for this policy year (USDC, 6 decimals)
    /// Used to calculate aggregate stop-loss trigger
    pub expected_annual_claims: u64,
    
    /// Year-to-date claims paid (USDC, 6 decimals)
    pub ytd_claims_paid: u64,
    
    /// Year-to-date recoveries received (USDC, 6 decimals)
    pub ytd_recoveries_received: u64,
    
    /// Total treaties registered
    pub total_treaties: u32,
    
    /// Active treaties count
    pub active_treaties: u32,
    
    /// Total recovery claims filed
    pub total_recovery_claims: u64,
    
    /// Total pending recoveries (USDC, 6 decimals)
    pub pending_recoveries: u64,
    
    /// Reinsurance premium paid YTD (USDC, 6 decimals)
    pub premium_paid_ytd: u64,
    
    /// Budget for reinsurance premium this year (USDC, 6 decimals)
    pub premium_budget: u64,
    
    /// Whether aggregate stop-loss has been triggered this year
    pub aggregate_triggered: bool,
    
    /// Whether catastrophic layer has been triggered
    pub catastrophic_triggered: bool,
    
    /// Claims ratio at which aggregate triggers (basis points, e.g., 12000 = 120%)
    pub aggregate_trigger_ratio_bps: u16,
    
    /// Claims ratio at which catastrophic triggers (basis points, e.g., 15000 = 150%)
    pub catastrophic_trigger_ratio_bps: u16,
    
    /// Maximum claims ratio covered by catastrophic (basis points, e.g., 30000 = 300%)
    pub catastrophic_ceiling_ratio_bps: u16,
    
    /// Bump seed for PDA
    pub bump: u8,
    
    /// Reserved for future use (split to avoid Default trait limitation)
    pub _reserved1: [u8; 32],
    pub _reserved2: [u8; 32],
}

impl ReinsuranceConfig {
    pub const SIZE: usize = 8 + // discriminator
        32 + // authority
        32 + // reinsurance_committee
        8 + // policy_year_start
        8 + // policy_year_end
        8 + // expected_annual_claims
        8 + // ytd_claims_paid
        8 + // ytd_recoveries_received
        4 + // total_treaties
        4 + // active_treaties
        8 + // total_recovery_claims
        8 + // pending_recoveries
        8 + // premium_paid_ytd
        8 + // premium_budget
        1 + // aggregate_triggered
        1 + // catastrophic_triggered
        2 + // aggregate_trigger_ratio_bps
        2 + // catastrophic_trigger_ratio_bps
        2 + // catastrophic_ceiling_ratio_bps
        1 + // bump
        64; // reserved
        
    /// Calculate current claims ratio in basis points
    pub fn current_claims_ratio_bps(&self) -> u64 {
        if self.expected_annual_claims == 0 {
            return 0;
        }
        // (ytd_claims * 10000) / expected
        self.ytd_claims_paid
            .checked_mul(10_000)
            .unwrap_or(0)
            .checked_div(self.expected_annual_claims)
            .unwrap_or(0)
    }
    
    /// Check if aggregate stop-loss should trigger
    pub fn should_trigger_aggregate(&self) -> bool {
        !self.aggregate_triggered && 
        self.current_claims_ratio_bps() >= self.aggregate_trigger_ratio_bps as u64
    }
    
    /// Check if catastrophic layer should trigger
    pub fn should_trigger_catastrophic(&self) -> bool {
        !self.catastrophic_triggered &&
        self.current_claims_ratio_bps() >= self.catastrophic_trigger_ratio_bps as u64
    }
    
    /// Calculate recoverable amount under aggregate layer
    /// Returns amount above trigger threshold (USDC)
    pub fn calculate_aggregate_recoverable(&self) -> u64 {
        if !self.aggregate_triggered {
            return 0;
        }
        
        // Trigger amount = expected * (trigger_ratio / 10000)
        let trigger_amount = self.expected_annual_claims
            .checked_mul(self.aggregate_trigger_ratio_bps as u64)
            .unwrap_or(0)
            .checked_div(10_000)
            .unwrap_or(0);
        
        // Ceiling amount for catastrophic layer
        let catastrophic_amount = self.expected_annual_claims
            .checked_mul(self.catastrophic_trigger_ratio_bps as u64)
            .unwrap_or(0)
            .checked_div(10_000)
            .unwrap_or(0);
            
        if self.ytd_claims_paid <= trigger_amount {
            return 0;
        }
        
        // Amount recoverable is excess above trigger, up to catastrophic threshold
        let excess = self.ytd_claims_paid.saturating_sub(trigger_amount);
        if self.catastrophic_triggered {
            // If catastrophic also triggered, aggregate only covers up to that threshold
            catastrophic_amount.saturating_sub(trigger_amount).min(excess)
        } else {
            excess
        }
    }
}

/// ============================================================================
/// REINSURANCE TREATY
/// ============================================================================

/// Individual reinsurance treaty/contract
#[account]
#[derive(Default)]
pub struct ReinsuranceTreaty {
    /// Unique treaty identifier (sequential)
    pub treaty_id: u64,
    
    /// Type of reinsurance layer
    pub layer_type: ReinsuranceLayerType,
    
    /// Treaty status
    pub status: TreatyStatus,
    
    /// Reinsurer identifier (off-chain reference, hashed)
    pub reinsurer_id: [u8; 32],
    
    /// Treaty effective date
    pub effective_date: i64,
    
    /// Treaty expiration date
    pub expiration_date: i64,
    
    // === SPECIFIC STOP-LOSS PARAMETERS ===
    
    /// Attachment point - claims above this trigger coverage (USDC, 6 decimals)
    /// For specific: per-member threshold
    /// For aggregate: percentage of expected (stored as USDC equivalent)
    pub attachment_point: u64,
    
    /// Coinsurance rate - portion Apollo retains above attachment (basis points)
    /// E.g., 2000 = 20% retained by Apollo, 80% to reinsurer
    pub coinsurance_rate_bps: u16,
    
    /// Maximum coverage per occurrence/member (USDC, 6 decimals)
    /// 0 = unlimited
    pub coverage_limit: u64,
    
    // === AGGREGATE PARAMETERS ===
    
    /// For aggregate: trigger ratio as % of expected claims (basis points)
    pub trigger_ratio_bps: u16,
    
    /// For aggregate: ceiling ratio (basis points)
    pub ceiling_ratio_bps: u16,
    
    // === FINANCIAL TRACKING ===
    
    /// Annual premium for this treaty (USDC, 6 decimals)
    pub annual_premium: u64,
    
    /// Premium paid to date (USDC, 6 decimals)
    pub premium_paid: u64,
    
    /// Total claims submitted under this treaty (USDC, 6 decimals)
    pub total_claims_submitted: u64,
    
    /// Total recoveries received (USDC, 6 decimals)
    pub total_recoveries_received: u64,
    
    /// Number of recovery claims filed
    pub recovery_claims_count: u32,
    
    /// Number of claims settled
    pub claims_settled_count: u32,
    
    /// Number of claims pending
    pub claims_pending_count: u32,
    
    // === METADATA ===
    
    /// Authority that can modify this treaty
    pub authority: Pubkey,
    
    /// Last updated timestamp
    pub last_updated: i64,
    
    /// Notes/reference (hashed)
    pub notes_hash: [u8; 32],
    
    /// Bump seed
    pub bump: u8,
    
    /// Reserved
    pub _reserved: [u8; 32],
}

impl ReinsuranceTreaty {
    pub const SIZE: usize = 8 + // discriminator
        8 + // treaty_id
        1 + // layer_type
        1 + // status
        32 + // reinsurer_id
        8 + // effective_date
        8 + // expiration_date
        8 + // attachment_point
        2 + // coinsurance_rate_bps
        8 + // coverage_limit
        2 + // trigger_ratio_bps
        2 + // ceiling_ratio_bps
        8 + // annual_premium
        8 + // premium_paid
        8 + // total_claims_submitted
        8 + // total_recoveries_received
        4 + // recovery_claims_count
        4 + // claims_settled_count
        4 + // claims_pending_count
        32 + // authority
        8 + // last_updated
        32 + // notes_hash
        1 + // bump
        32; // reserved
        
    /// Check if treaty is currently active
    pub fn is_active(&self, current_time: i64) -> bool {
        self.status == TreatyStatus::Active &&
        current_time >= self.effective_date &&
        current_time <= self.expiration_date
    }
    
    /// Calculate coverage amount for a claim exceeding attachment
    /// Returns (apollo_portion, reinsurer_portion)
    pub fn calculate_coverage(&self, excess_amount: u64) -> (u64, u64) {
        // Apollo retains coinsurance_rate_bps / 10000
        let apollo_portion = excess_amount
            .checked_mul(self.coinsurance_rate_bps as u64)
            .unwrap_or(0)
            .checked_div(10_000)
            .unwrap_or(0);
            
        let mut reinsurer_portion = excess_amount.saturating_sub(apollo_portion);
        
        // Apply coverage limit if set
        if self.coverage_limit > 0 {
            reinsurer_portion = reinsurer_portion.min(self.coverage_limit);
        }
        
        (apollo_portion, reinsurer_portion)
    }
}

impl Default for ReinsuranceLayerType {
    fn default() -> Self {
        ReinsuranceLayerType::SpecificStopLoss
    }
}

/// ============================================================================
/// RECOVERY CLAIM
/// ============================================================================

/// Individual recovery claim filed against a treaty
#[account]
#[derive(Default)]
pub struct RecoveryClaim {
    /// Unique claim identifier
    pub claim_id: u64,
    
    /// Treaty this claim is filed against
    pub treaty: Pubkey,
    
    /// Layer type (cached from treaty)
    pub layer_type: ReinsuranceLayerType,
    
    /// Claim status
    pub status: RecoveryStatus,
    
    /// For specific stop-loss: the member whose claims triggered this
    /// (hashed for privacy)
    pub member_hash: [u8; 32],
    
    /// Reference to original claim(s) in apollo_claims
    pub original_claim_ids: [u64; 8],
    
    /// Number of original claims referenced
    pub original_claims_count: u8,
    
    // === FINANCIAL DATA ===
    
    /// Total claims amount that triggered this recovery (USDC)
    pub total_claims_amount: u64,
    
    /// Attachment point at time of filing (USDC)
    pub attachment_point: u64,
    
    /// Amount above attachment (excess)
    pub excess_amount: u64,
    
    /// Apollo's coinsurance portion (USDC)
    pub apollo_portion: u64,
    
    /// Amount claimed from reinsurer (USDC)
    pub claimed_amount: u64,
    
    /// Amount approved by reinsurer (USDC)
    pub approved_amount: u64,
    
    /// Amount actually received (USDC)
    pub received_amount: u64,
    
    // === TIMESTAMPS ===
    
    /// When the triggering event occurred
    pub event_timestamp: i64,
    
    /// When the recovery claim was filed
    pub filed_timestamp: i64,
    
    /// When submitted to reinsurer
    pub submitted_timestamp: i64,
    
    /// When approved/denied
    pub resolution_timestamp: i64,
    
    /// When payment received
    pub settlement_timestamp: i64,
    
    // === METADATA ===
    
    /// Who filed this claim
    pub filed_by: Pubkey,
    
    /// Supporting documentation hash
    pub documentation_hash: [u8; 32],
    
    /// Reinsurer's reference number (if provided)
    pub reinsurer_reference: [u8; 32],
    
    /// Resolution notes hash
    pub resolution_notes_hash: [u8; 32],
    
    /// Bump seed
    pub bump: u8,
    
    /// Reserved
    pub _reserved: [u8; 16],
}

impl RecoveryClaim {
    pub const SIZE: usize = 8 + // discriminator
        8 + // claim_id
        32 + // treaty
        1 + // layer_type
        1 + // status
        32 + // member_hash
        64 + // original_claim_ids (8 * 8)
        1 + // original_claims_count
        8 + // total_claims_amount
        8 + // attachment_point
        8 + // excess_amount
        8 + // apollo_portion
        8 + // claimed_amount
        8 + // approved_amount
        8 + // received_amount
        8 + // event_timestamp
        8 + // filed_timestamp
        8 + // submitted_timestamp
        8 + // resolution_timestamp
        8 + // settlement_timestamp
        32 + // filed_by
        32 + // documentation_hash
        32 + // reinsurer_reference
        32 + // resolution_notes_hash
        1 + // bump
        16; // reserved
}

/// ============================================================================
/// MEMBER ACCUMULATOR FOR SPECIFIC STOP-LOSS
/// ============================================================================

/// Tracks per-member claims accumulation for specific stop-loss
#[account]
#[derive(Default)]
pub struct MemberClaimsAccumulator {
    /// Member identifier (pubkey or hash)
    pub member: Pubkey,
    
    /// Policy year this accumulator covers
    pub policy_year: u16,
    
    /// Year-to-date claims for this member (USDC)
    pub ytd_claims: u64,
    
    /// Claims count this year
    pub claims_count: u32,
    
    /// Amount that exceeded stop-loss attachment
    pub excess_claimed: u64,
    
    /// Amount recovered from reinsurer
    pub recovered_amount: u64,
    
    /// Highest single claim amount
    pub max_single_claim: u64,
    
    /// Whether stop-loss has been triggered
    pub stop_loss_triggered: bool,
    
    /// First trigger timestamp
    pub first_trigger_timestamp: i64,
    
    /// Last claim timestamp
    pub last_claim_timestamp: i64,
    
    /// Bump seed
    pub bump: u8,
    
    /// Reserved
    pub _reserved: [u8; 32],
}

impl MemberClaimsAccumulator {
    pub const SIZE: usize = 8 + // discriminator
        32 + // member
        2 + // policy_year
        8 + // ytd_claims
        4 + // claims_count
        8 + // excess_claimed
        8 + // recovered_amount
        8 + // max_single_claim
        1 + // stop_loss_triggered
        8 + // first_trigger_timestamp
        8 + // last_claim_timestamp
        1 + // bump
        32; // reserved
        
    /// Check if a new claim should trigger stop-loss
    pub fn check_stop_loss_trigger(&self, new_claim_amount: u64, attachment: u64) -> Option<u64> {
        let new_total = self.ytd_claims.saturating_add(new_claim_amount);
        if new_total > attachment {
            // Calculate excess - could be partial (just this claim) or include prior
            if self.ytd_claims >= attachment {
                // Already over, entire new claim is excess
                Some(new_claim_amount)
            } else {
                // This claim pushes over the threshold
                Some(new_total.saturating_sub(attachment))
            }
        } else {
            None
        }
    }
}

/// ============================================================================
/// AGGREGATE TRACKING
/// ============================================================================

/// Monthly aggregate claims tracking for trend analysis
#[account]
#[derive(Default)]
pub struct MonthlyAggregate {
    /// Policy year
    pub policy_year: u16,
    
    /// Month (1-12)
    pub month: u8,
    
    /// Total claims this month (USDC)
    pub total_claims: u64,
    
    /// Number of claims
    pub claims_count: u32,
    
    /// Average claim amount
    pub avg_claim_amount: u64,
    
    /// Expected claims for this month (USDC)
    pub expected_claims: u64,
    
    /// Ratio of actual to expected (basis points)
    pub ratio_bps: u16,
    
    /// Running YTD total through this month
    pub ytd_through_month: u64,
    
    /// Largest single claim this month
    pub max_claim: u64,
    
    /// Number of shock claims (>$50k)
    pub shock_claims_count: u16,
    
    /// Timestamp of last update
    pub last_updated: i64,
    
    /// Bump seed
    pub bump: u8,
    
    /// Reserved
    pub _reserved: [u8; 16],
}

impl MonthlyAggregate {
    pub const SIZE: usize = 8 + // discriminator
        2 + // policy_year
        1 + // month
        8 + // total_claims
        4 + // claims_count
        8 + // avg_claim_amount
        8 + // expected_claims
        2 + // ratio_bps
        8 + // ytd_through_month
        8 + // max_claim
        2 + // shock_claims_count
        8 + // last_updated
        1 + // bump
        16; // reserved
}

/// ============================================================================
/// SCALABLE REINSURANCE CONFIGURATION (Small-Scale Viability)
/// ============================================================================
///
/// Reinsurance parameters that scale with pool size to maintain appropriate
/// risk transfer for both small ($1.5M capital) and large ($50M+) pools.
///
/// Key insight: Small pools have higher variance (law of large numbers)
/// Therefore, small pools need:
/// 1. Lower specific stop-loss attachment (more claims covered)
/// 2. Lower aggregate stop-loss trigger (earlier protection)
/// 3. Higher reinsurance budget (as % of premiums)

/// Pool size thresholds for parameter scaling
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct PoolSizeThresholds {
    /// Small pool: < this many members
    pub small_pool_max: u64,
    /// Medium pool: < this many members (above this = large)
    pub medium_pool_max: u64,
}

impl Default for PoolSizeThresholds {
    fn default() -> Self {
        Self {
            small_pool_max: 500,
            medium_pool_max: 5000,
        }
    }
}

/// Scalable reinsurance parameters based on pool size
/// Used to determine appropriate protection levels
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct ScalableReinsuranceParams {
    /// Pool size thresholds
    pub thresholds: PoolSizeThresholds,
    
    // ===== Specific Stop-Loss Attachment Points =====
    // Lower attachment = more claims covered = higher premiums
    
    /// Small pool attachment (USDC, 6 decimals)
    /// Default: $50,000 - protects against high-cost individuals
    pub small_pool_attachment: u64,
    
    /// Medium pool attachment (USDC, 6 decimals)
    /// Default: $75,000
    pub medium_pool_attachment: u64,
    
    /// Large pool attachment (USDC, 6 decimals)
    /// Default: $100,000
    pub large_pool_attachment: u64,
    
    // ===== Aggregate Stop-Loss Trigger Ratios =====
    // Lower trigger = earlier protection = higher premiums
    
    /// Small pool aggregate trigger (basis points of expected claims)
    /// Default: 105% - triggers early due to high variance
    pub small_pool_aggregate_trigger_bps: u16,
    
    /// Medium pool aggregate trigger
    /// Default: 108%
    pub medium_pool_aggregate_trigger_bps: u16,
    
    /// Large pool aggregate trigger  
    /// Default: 110%
    pub large_pool_aggregate_trigger_bps: u16,
    
    // ===== Reinsurance Budget (% of premiums) =====
    // Recommended allocation for reinsurance costs
    
    /// Small pool budget (basis points of total premiums)
    /// Default: 20% - higher cost for better protection
    pub small_pool_budget_bps: u16,
    
    /// Medium pool budget
    /// Default: 12%
    pub medium_pool_budget_bps: u16,
    
    /// Large pool budget
    /// Default: 8%
    pub large_pool_budget_bps: u16,
    
    // ===== Catastrophic Coverage =====
    
    /// Catastrophic trigger ratio for all pool sizes
    /// Default: 150% of expected claims
    pub catastrophic_trigger_bps: u16,
    
    /// Catastrophic ceiling ratio
    /// Default: 300% of expected claims
    pub catastrophic_ceiling_bps: u16,
}

impl Default for ScalableReinsuranceParams {
    fn default() -> Self {
        Self {
            thresholds: PoolSizeThresholds::default(),
            
            // Specific stop-loss attachments (USDC with 6 decimals)
            small_pool_attachment: 50_000_000_000,   // $50,000
            medium_pool_attachment: 75_000_000_000,  // $75,000
            large_pool_attachment: 100_000_000_000,  // $100,000
            
            // Aggregate triggers (basis points)
            small_pool_aggregate_trigger_bps: 10500,  // 105%
            medium_pool_aggregate_trigger_bps: 10800, // 108%
            large_pool_aggregate_trigger_bps: 11000,  // 110%
            
            // Budget allocations (basis points of premiums)
            small_pool_budget_bps: 2000,  // 20%
            medium_pool_budget_bps: 1200, // 12%
            large_pool_budget_bps: 800,   // 8%
            
            // Catastrophic (same for all sizes)
            catastrophic_trigger_bps: 15000, // 150%
            catastrophic_ceiling_bps: 30000, // 300%
        }
    }
}

impl ScalableReinsuranceParams {
    /// Get appropriate specific stop-loss attachment for pool size
    pub fn get_attachment(&self, member_count: u64) -> u64 {
        if member_count < self.thresholds.small_pool_max {
            self.small_pool_attachment
        } else if member_count < self.thresholds.medium_pool_max {
            self.medium_pool_attachment
        } else {
            self.large_pool_attachment
        }
    }
    
    /// Get appropriate aggregate trigger ratio for pool size
    pub fn get_aggregate_trigger_bps(&self, member_count: u64) -> u16 {
        if member_count < self.thresholds.small_pool_max {
            self.small_pool_aggregate_trigger_bps
        } else if member_count < self.thresholds.medium_pool_max {
            self.medium_pool_aggregate_trigger_bps
        } else {
            self.large_pool_aggregate_trigger_bps
        }
    }
    
    /// Get recommended reinsurance budget for pool size (as % of premiums)
    pub fn get_budget_bps(&self, member_count: u64) -> u16 {
        if member_count < self.thresholds.small_pool_max {
            self.small_pool_budget_bps
        } else if member_count < self.thresholds.medium_pool_max {
            self.medium_pool_budget_bps
        } else {
            self.large_pool_budget_bps
        }
    }
    
    /// Calculate recommended annual reinsurance budget from expected premiums
    pub fn calculate_annual_budget(&self, member_count: u64, annual_premiums: u64) -> u64 {
        let budget_bps = self.get_budget_bps(member_count);
        annual_premiums
            .saturating_mul(budget_bps as u64)
            .checked_div(10000)
            .unwrap_or(0)
    }
    
    /// Get pool size category as string (for logging/display)
    pub fn get_pool_category(&self, member_count: u64) -> &'static str {
        if member_count < self.thresholds.small_pool_max {
            "small"
        } else if member_count < self.thresholds.medium_pool_max {
            "medium"
        } else {
            "large"
        }
    }
}

/// ============================================================================
/// UNIT TESTS FOR SCALABLE PARAMS
/// ============================================================================

#[cfg(test)]
mod scalable_tests {
    use super::*;
    
    #[test]
    fn test_pool_size_thresholds() {
        let thresholds = PoolSizeThresholds::default();
        assert_eq!(thresholds.small_pool_max, 500);
        assert_eq!(thresholds.medium_pool_max, 5000);
    }
    
    #[test]
    fn test_scalable_params_defaults() {
        let params = ScalableReinsuranceParams::default();
        
        // Attachments
        assert_eq!(params.small_pool_attachment, 50_000_000_000);
        assert_eq!(params.medium_pool_attachment, 75_000_000_000);
        assert_eq!(params.large_pool_attachment, 100_000_000_000);
        
        // Triggers
        assert_eq!(params.small_pool_aggregate_trigger_bps, 10500);
        assert_eq!(params.large_pool_aggregate_trigger_bps, 11000);
        
        // Budgets
        assert_eq!(params.small_pool_budget_bps, 2000); // 20%
        assert_eq!(params.large_pool_budget_bps, 800);  // 8%
    }
    
    #[test]
    fn test_get_attachment_small_pool() {
        let params = ScalableReinsuranceParams::default();
        
        // 200 members = small pool
        let attachment = params.get_attachment(200);
        assert_eq!(attachment, 50_000_000_000); // $50k
    }
    
    #[test]
    fn test_get_attachment_medium_pool() {
        let params = ScalableReinsuranceParams::default();
        
        // 1000 members = medium pool
        let attachment = params.get_attachment(1000);
        assert_eq!(attachment, 75_000_000_000); // $75k
    }
    
    #[test]
    fn test_get_attachment_large_pool() {
        let params = ScalableReinsuranceParams::default();
        
        // 10000 members = large pool
        let attachment = params.get_attachment(10000);
        assert_eq!(attachment, 100_000_000_000); // $100k
    }
    
    #[test]
    fn test_get_aggregate_trigger() {
        let params = ScalableReinsuranceParams::default();
        
        // Small pool gets lower trigger (earlier protection)
        assert_eq!(params.get_aggregate_trigger_bps(100), 10500);
        
        // Large pool gets higher trigger
        assert_eq!(params.get_aggregate_trigger_bps(10000), 11000);
    }
    
    #[test]
    fn test_calculate_annual_budget() {
        let params = ScalableReinsuranceParams::default();
        
        // Small pool: 200 members, $1M annual premiums
        // Budget should be 20% = $200k
        let budget = params.calculate_annual_budget(200, 1_000_000_000_000);
        assert_eq!(budget, 200_000_000_000); // $200k
        
        // Large pool: 10k members, $50M annual premiums
        // Budget should be 8% = $4M
        let budget = params.calculate_annual_budget(10000, 50_000_000_000_000);
        assert_eq!(budget, 4_000_000_000_000); // $4M
    }
    
    #[test]
    fn test_pool_category() {
        let params = ScalableReinsuranceParams::default();
        
        assert_eq!(params.get_pool_category(100), "small");
        assert_eq!(params.get_pool_category(500), "medium"); // At threshold
        assert_eq!(params.get_pool_category(1000), "medium");
        assert_eq!(params.get_pool_category(5000), "large"); // At threshold
        assert_eq!(params.get_pool_category(10000), "large");
    }
}

