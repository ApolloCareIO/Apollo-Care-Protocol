// programs/apollo_claims/src/ai_oracle.rs
//
// AI/ML Claims Processing Oracle
// Implements the three-tier claims processing architecture:
// 1. Fast-Lane Auto-Approval (small routine claims)
// 2. AI-Assisted Triage (medium claims with ML scoring)
// 3. Committee Escalation (large/complex claims)

use anchor_lang::prelude::*;

/// AI Claims Oracle Configuration
/// PDA seeds: ["claims_oracle"]
#[account]
#[derive(InitSpace)]
pub struct ClaimsOracle {
    /// Authority for oracle management
    pub authority: Pubkey,
    
    /// Authorized oracle signers (off-chain AI service keys)
    #[max_len(5)]
    pub authorized_signers: Vec<Pubkey>,
    
    /// Required signatures for AI decision submission
    pub required_sigs: u8,
    
    /// Total decisions processed
    pub total_decisions: u64,
    
    /// Decisions auto-approved by AI
    pub auto_approved: u64,
    
    /// Decisions auto-denied by AI
    pub auto_denied: u64,
    
    /// Decisions escalated to committee
    pub escalated: u64,
    
    /// Decisions overturned by committee (tracks AI accuracy)
    pub decisions_overturned: u64,
    
    /// Oracle accuracy rate (bps) - computed from overturns
    pub accuracy_rate_bps: u16,
    
    /// Minimum confidence for auto-approve (bps, e.g., 9500 = 95%)
    pub auto_approve_confidence_bps: u16,
    
    /// Minimum confidence for auto-deny (bps)
    pub auto_deny_confidence_bps: u16,
    
    /// Maximum fraud score for auto-approve (bps, e.g., 3000 = 30%)
    pub max_fraud_score_for_approve_bps: u16,
    
    /// Is oracle active
    pub is_active: bool,
    
    /// Last decision timestamp
    pub last_decision_at: i64,
    
    /// Bump seed
    pub bump: u8,
}

impl ClaimsOracle {
    pub const SEED_PREFIX: &'static [u8] = b"claims_oracle";
    
    // Default thresholds for AI decision-making
    pub const DEFAULT_AUTO_APPROVE_CONFIDENCE: u16 = 9500; // 95%
    pub const DEFAULT_AUTO_DENY_CONFIDENCE: u16 = 9500; // 95%
    pub const DEFAULT_MAX_FRAUD_FOR_APPROVE: u16 = 3000; // 30%
    
    /// Check if a signer is authorized
    pub fn is_authorized(&self, signer: &Pubkey) -> bool {
        self.authorized_signers.contains(signer)
    }
    
    /// Update accuracy rate based on overturns
    pub fn update_accuracy(&mut self) {
        let total_final = self.auto_approved + self.auto_denied;
        if total_final == 0 {
            self.accuracy_rate_bps = 10000; // 100% if no decisions
            return;
        }
        
        let correct = total_final.saturating_sub(self.decisions_overturned);
        self.accuracy_rate_bps = (correct
            .saturating_mul(10000)
            .checked_div(total_final)
            .unwrap_or(10000)) as u16;
    }
}

/// AI Decision record for a specific claim
/// PDA seeds: ["ai_decision", claim_id]
#[account]
#[derive(InitSpace)]
pub struct AiDecision {
    /// Claim ID this decision is for
    pub claim_id: u64,
    
    /// Decision type
    pub decision: AiDecisionType,
    
    /// Overall confidence score (bps)
    pub confidence_bps: u16,
    
    /// Price reasonableness score (bps, 10000 = perfectly reasonable)
    pub price_score_bps: u16,
    
    /// Fraud risk score (bps, 0 = no risk, 10000 = definite fraud)
    pub fraud_score_bps: u16,
    
    /// Procedure-diagnosis consistency score (bps)
    pub consistency_score_bps: u16,
    
    /// Flags raised by AI analysis
    #[max_len(5)]
    pub flags: Vec<AiFlag>,
    
    /// Suggested approved amount (may differ from requested)
    pub suggested_amount: u64,
    
    /// Reference price from UCR database
    pub reference_price: u64,
    
    /// Oracle signer who submitted this decision
    pub submitted_by: Pubkey,
    
    /// Submission timestamp
    pub submitted_at: i64,
    
    /// Was this decision overturned by committee
    pub overturned: bool,
    
    /// Overturn reason (if applicable)
    #[max_len(128)]
    pub overturn_reason: String,
    
    /// Bump seed
    pub bump: u8,
}

impl AiDecision {
    pub const SEED_PREFIX: &'static [u8] = b"ai_decision";
}

/// AI Decision types
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum AiDecisionType {
    /// Auto-approve: High confidence, low fraud risk
    AutoApprove,
    /// Auto-deny: High confidence fraud or clear policy violation
    AutoDeny,
    /// Escalate to committee: Medium confidence or complex case
    CommitteeReview,
    /// Need more information from member
    RequestInfo,
}

impl Default for AiDecisionType {
    fn default() -> Self {
        AiDecisionType::CommitteeReview
    }
}

/// AI-generated flags for claims
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum AiFlag {
    /// Price significantly above UCR
    PriceAboveUcr,
    /// Price significantly below UCR (suspicious)
    PriceBelowUcr,
    /// Procedure doesn't match diagnosis
    ProcedureDiagnosisMismatch,
    /// Duplicate claim detected
    DuplicateClaim,
    /// Unusual claim frequency
    HighFrequency,
    /// Provider on watch list
    ProviderWatchList,
    /// Member in waiting period
    WaitingPeriod,
    /// Benefit limit exceeded
    BenefitLimitExceeded,
    /// Missing documentation
    MissingDocumentation,
    /// New member (< 30 days)
    NewMember,
    /// Experimental treatment
    ExperimentalTreatment,
    /// Out of network
    OutOfNetwork,
    /// Pre-authorization required
    PreAuthRequired,
    /// Coordination of benefits needed
    CobRequired,
}

/// Fast-lane eligibility configuration
/// PDA seeds: ["fast_lane_config"]
#[account]
#[derive(InitSpace)]
pub struct FastLaneConfig {
    /// Authority
    pub authority: Pubkey,
    
    /// Maximum amount for fast-lane (USDC)
    pub max_amount: u64,
    
    /// Eligible claim categories for fast-lane
    #[max_len(10)]
    pub eligible_categories: Vec<u8>, // ClaimCategory as u8
    
    /// Maximum fast-lane claims per member per 30-day period
    pub max_claims_per_period: u8,
    
    /// Period length in seconds (default: 30 days)
    pub period_seconds: i64,
    
    /// Minimum member tenure for fast-lane (days)
    pub min_member_tenure_days: u16,
    
    /// Is fast-lane active
    pub is_active: bool,
    
    /// Total fast-lane approvals
    pub total_fast_lane_approvals: u64,
    
    /// Total amount paid via fast-lane
    pub total_fast_lane_paid: u64,
    
    /// Bump seed
    pub bump: u8,
}

impl FastLaneConfig {
    pub const SEED_PREFIX: &'static [u8] = b"fast_lane_config";
    
    // Bootstrap defaults (more conservative)
    pub const BOOTSTRAP_MAX_AMOUNT: u64 = 500_000_000; // $500
    pub const BOOTSTRAP_MAX_CLAIMS: u8 = 3;
    
    // Standard defaults
    pub const STANDARD_MAX_AMOUNT: u64 = 1_000_000_000; // $1,000
    pub const STANDARD_MAX_CLAIMS: u8 = 5;
    
    pub const DEFAULT_PERIOD_SECONDS: i64 = 30 * 24 * 60 * 60; // 30 days
    pub const DEFAULT_MIN_TENURE_DAYS: u16 = 30;
}

/// Member fast-lane tracking
/// PDA seeds: ["fast_lane_tracker", member]
#[account]
#[derive(InitSpace)]
pub struct FastLaneTracker {
    /// Member pubkey
    pub member: Pubkey,
    
    /// Claims in current period
    pub claims_this_period: u8,
    
    /// Period start timestamp
    pub period_start: i64,
    
    /// Total fast-lane claims all-time
    pub total_fast_lane_claims: u32,
    
    /// Total amount received via fast-lane
    pub total_fast_lane_amount: u64,
    
    /// Is member flagged (excessive fast-lane usage)
    pub flagged: bool,
    
    /// Bump seed
    pub bump: u8,
}

impl FastLaneTracker {
    pub const SEED_PREFIX: &'static [u8] = b"fast_lane_tracker";
    
    /// Check if member can use fast-lane
    pub fn can_use_fast_lane(&self, config: &FastLaneConfig, current_time: i64) -> bool {
        if self.flagged {
            return false;
        }
        
        // Check if we're in a new period
        if current_time >= self.period_start + config.period_seconds {
            return true; // New period, reset counter
        }
        
        // Check claims limit
        self.claims_this_period < config.max_claims_per_period
    }
    
    /// Record a fast-lane claim
    pub fn record_claim(&mut self, amount: u64, config: &FastLaneConfig, current_time: i64) {
        // Reset period if needed
        if current_time >= self.period_start + config.period_seconds {
            self.period_start = current_time;
            self.claims_this_period = 0;
        }
        
        self.claims_this_period += 1;
        self.total_fast_lane_claims += 1;
        self.total_fast_lane_amount += amount;
    }
}

/// UCR (Usual, Customary, Reasonable) Price Reference
/// PDA seeds: ["ucr_reference", procedure_code]
#[account]
#[derive(InitSpace)]
pub struct UcrReference {
    /// Procedure code (CPT/HCPCS)
    #[max_len(10)]
    pub procedure_code: String,
    
    /// Base reference price (USDC, national average)
    pub base_price: u64,
    
    /// Low end of reasonable range (USDC)
    pub price_low: u64,
    
    /// High end of reasonable range (USDC)
    pub price_high: u64,
    
    /// Regional adjustments (region_code -> factor_bps)
    #[max_len(20)]
    pub regional_factors: Vec<RegionalPriceFactor>,
    
    /// Last updated timestamp
    pub last_updated: i64,
    
    /// Data source hash (for audit)
    #[max_len(64)]
    pub source_hash: String,
    
    /// Bump seed
    pub bump: u8,
}

impl UcrReference {
    pub const SEED_PREFIX: &'static [u8] = b"ucr_reference";
    
    /// Get price range for a region
    pub fn get_regional_range(&self, region_code: u8) -> (u64, u64) {
        let factor = self.regional_factors
            .iter()
            .find(|r| r.region_code == region_code)
            .map(|r| r.factor_bps)
            .unwrap_or(10000); // Default 1.0x
        
        let low = (self.price_low as u128 * factor as u128 / 10000) as u64;
        let high = (self.price_high as u128 * factor as u128 / 10000) as u64;
        
        (low, high)
    }
    
    /// Check if a price is within reasonable range
    pub fn is_price_reasonable(&self, price: u64, region_code: u8) -> (bool, u16) {
        let (low, high) = self.get_regional_range(region_code);
        
        if price >= low && price <= high {
            return (true, 10000); // 100% score
        }
        
        // Calculate how far outside range
        if price < low {
            let ratio = (price as u128 * 10000 / low as u128) as u16;
            (ratio >= 5000, ratio) // Reasonable if within 50%
        } else {
            let ratio = (high as u128 * 10000 / price as u128) as u16;
            (ratio >= 5000, ratio)
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct RegionalPriceFactor {
    pub region_code: u8,
    pub factor_bps: u16,
}

// =============================================================================
// AI DECISION LOGIC (called by oracle service)
// =============================================================================

/// Evaluate a claim and produce an AI decision
/// This is the core logic that the off-chain oracle uses
pub fn evaluate_claim_for_decision(
    claim_amount: u64,
    claim_category: u8,
    member_tenure_days: u16,
    price_score: u16,      // From UCR comparison
    fraud_indicators: u8,   // Count of fraud flags
    has_documentation: bool,
    is_duplicate: bool,
    oracle_config: &ClaimsOracle,
    fast_lane_config: &FastLaneConfig,
) -> (AiDecisionType, u16, Vec<AiFlag>) {
    let mut flags = Vec::new();
    let mut confidence: u32 = 10000; // Start at 100%
    
    // Check for auto-deny conditions first
    if is_duplicate {
        flags.push(AiFlag::DuplicateClaim);
        return (AiDecisionType::AutoDeny, 9800, flags);
    }
    
    // New member flag
    if member_tenure_days < fast_lane_config.min_member_tenure_days {
        flags.push(AiFlag::NewMember);
        confidence = confidence.saturating_sub(1500);
    }
    
    // Price reasonableness
    if price_score < 5000 {
        flags.push(AiFlag::PriceAboveUcr);
        confidence = confidence.saturating_sub(2000);
    } else if price_score < 7000 {
        confidence = confidence.saturating_sub(1000);
    }
    
    // Documentation
    if !has_documentation {
        flags.push(AiFlag::MissingDocumentation);
        confidence = confidence.saturating_sub(2000);
    }
    
    // Fraud indicators
    let fraud_score = (fraud_indicators as u32 * 2000).min(10000);
    confidence = confidence.saturating_sub(fraud_score / 2);
    
    // Calculate final confidence
    let final_confidence = confidence.min(10000) as u16;
    
    // Decision logic
    let fraud_score_bps = (fraud_indicators as u16 * 2000).min(10000);
    
    if fraud_score_bps >= oracle_config.auto_deny_confidence_bps {
        (AiDecisionType::AutoDeny, final_confidence, flags)
    } else if final_confidence >= oracle_config.auto_approve_confidence_bps 
        && fraud_score_bps <= oracle_config.max_fraud_score_for_approve_bps 
        && flags.is_empty() 
    {
        (AiDecisionType::AutoApprove, final_confidence, flags)
    } else if !has_documentation {
        (AiDecisionType::RequestInfo, final_confidence, flags)
    } else {
        (AiDecisionType::CommitteeReview, final_confidence, flags)
    }
}

// =============================================================================
// EVENTS
// =============================================================================

#[event]
pub struct AiDecisionSubmitted {
    pub claim_id: u64,
    pub decision: AiDecisionType,
    pub confidence_bps: u16,
    pub fraud_score_bps: u16,
    pub flags_count: u8,
    pub oracle_signer: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct AiDecisionOverturned {
    pub claim_id: u64,
    pub original_decision: AiDecisionType,
    pub new_decision: AiDecisionType,
    pub overturn_reason: String,
    pub overturned_by: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct FastLaneApproved {
    pub claim_id: u64,
    pub member: Pubkey,
    pub amount: u64,
    pub category: u8,
    pub timestamp: i64,
}
