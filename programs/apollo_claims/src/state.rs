// programs/apollo_claims/src/state.rs

use anchor_lang::prelude::*;

// =============================================================================
// AI ORACLE CONFIGURATION
// =============================================================================
// Off-chain AI service integration for claims processing

/// AI Oracle configuration
/// PDA seeds: ["ai_oracle_config"]
#[account]
#[derive(InitSpace)]
pub struct AiOracleConfig {
    /// Oracle authority (can submit AI decisions)
    pub oracle_authority: Pubkey,
    
    /// Backup oracle (if primary fails)
    pub backup_oracle: Pubkey,
    
    /// Maximum staleness for AI decisions (seconds)
    pub max_decision_age: i64,
    
    /// Auto-approve threshold score (0-100, lower = safer)
    /// Claims with risk score below this are auto-approved
    pub auto_approve_threshold: u8,
    
    /// Escalation threshold score (0-100, higher = riskier)
    /// Claims with risk score above this go to committee
    pub escalation_threshold: u8,
    
    /// Is AI processing enabled
    pub is_enabled: bool,
    
    /// Total decisions processed
    pub total_decisions: u64,
    
    /// Decisions overridden by committee
    pub overridden_count: u64,
    
    /// Bump seed
    pub bump: u8,
}

impl AiOracleConfig {
    pub const SEED_PREFIX: &'static [u8] = b"ai_oracle_config";
    
    pub const DEFAULT_MAX_DECISION_AGE: i64 = 3600; // 1 hour
    pub const DEFAULT_AUTO_APPROVE_THRESHOLD: u8 = 30;
    pub const DEFAULT_ESCALATION_THRESHOLD: u8 = 70;
}

/// AI decision for a specific claim
/// PDA seeds: ["ai_decision", claim_id]
#[account]
#[derive(InitSpace)]
pub struct AiDecision {
    /// Claim this decision applies to
    pub claim_id: u64,
    
    /// Risk score (0-100, higher = riskier)
    pub risk_score: u8,
    
    /// Fraud indicators detected
    pub fraud_flags: FraudFlags,
    
    /// Price reasonableness score (0-100, higher = more reasonable)
    pub price_score: u8,
    
    /// Recommended action
    pub recommendation: AiRecommendation,
    
    /// Recommended amount (may differ from requested)
    pub recommended_amount: u64,
    
    /// Confidence level (0-100)
    pub confidence: u8,
    
    /// Reasoning hash (IPFS link to detailed explanation)
    #[max_len(64)]
    pub reasoning_hash: String,
    
    /// Timestamp of decision
    pub decided_at: i64,
    
    /// Oracle that submitted this decision
    pub oracle: Pubkey,
    
    /// Was this decision overridden by committee
    pub was_overridden: bool,
    
    /// Bump seed
    pub bump: u8,
}

impl AiDecision {
    pub const SEED_PREFIX: &'static [u8] = b"ai_decision";
}

/// Fraud indicators detected by AI
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, InitSpace)]
pub struct FraudFlags {
    /// Duplicate claim detected
    pub duplicate_claim: bool,
    /// Price significantly above reference
    pub price_anomaly: bool,
    /// Unusual claim frequency
    pub frequency_anomaly: bool,
    /// Document issues detected (OCR confidence, alterations)
    pub document_issues: bool,
    /// Provider on watchlist
    pub provider_flagged: bool,
    /// Service doesn't match diagnosis
    pub service_mismatch: bool,
    /// Timing anomaly (e.g., claim before enrollment)
    pub timing_anomaly: bool,
    /// Geographic anomaly (service far from member location)
    pub geographic_anomaly: bool,
}

impl FraudFlags {
    /// Count number of flags set
    pub fn count(&self) -> u8 {
        let mut count = 0;
        if self.duplicate_claim { count += 1; }
        if self.price_anomaly { count += 1; }
        if self.frequency_anomaly { count += 1; }
        if self.document_issues { count += 1; }
        if self.provider_flagged { count += 1; }
        if self.service_mismatch { count += 1; }
        if self.timing_anomaly { count += 1; }
        if self.geographic_anomaly { count += 1; }
        count
    }
    
    /// Check if any critical flags are set
    pub fn has_critical(&self) -> bool {
        self.duplicate_claim || self.provider_flagged || self.document_issues
    }
}

/// AI recommendation for claim processing
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace, Default)]
pub enum AiRecommendation {
    /// Auto-approve in full
    AutoApproveFull,
    /// Auto-approve partial amount
    AutoApprovePartial,
    /// Needs human review (medium risk)
    #[default]
    NeedsReview,
    /// High risk - escalate to committee
    Escalate,
    /// Likely fraud - deny with investigation
    DenyFraud,
}

// =============================================================================
// REFERENCE PRICE DATABASE
// =============================================================================
// For cost reasonableness checks

/// Reference price entry for a service code in a region
/// PDA seeds: ["reference_price", service_code, region]
#[account]
#[derive(InitSpace)]
pub struct ReferencePrice {
    /// CPT/HCPCS code (e.g., "99213" for office visit)
    #[max_len(8)]
    pub service_code: String,
    
    /// Region code (0-255)
    pub region: u8,
    
    /// 25th percentile price (USDC lamports)
    pub p25_price: u64,
    
    /// 50th percentile (median) price
    pub p50_price: u64,
    
    /// 75th percentile price
    pub p75_price: u64,
    
    /// 95th percentile (high but acceptable)
    pub p95_price: u64,
    
    /// Last updated timestamp
    pub updated_at: i64,
    
    /// Data source identifier
    #[max_len(32)]
    pub source: String,
    
    /// Bump seed
    pub bump: u8,
}

impl ReferencePrice {
    pub const SEED_PREFIX: &'static [u8] = b"reference_price";
    
    /// Check if a price is reasonable (at or below 95th percentile)
    pub fn is_reasonable(&self, price: u64) -> bool {
        price <= self.p95_price
    }
    
    /// Get price reasonableness score (0-100, higher = more reasonable)
    pub fn get_price_score(&self, price: u64) -> u8 {
        if price <= self.p25_price {
            100 // Excellent price
        } else if price <= self.p50_price {
            85 // Good price
        } else if price <= self.p75_price {
            70 // Acceptable
        } else if price <= self.p95_price {
            50 // High but acceptable
        } else {
            // Above 95th percentile - score decreases further
            let excess_ratio = (price as u128 * 100) / (self.p95_price as u128);
            if excess_ratio > 200 {
                0 // 2x+ over 95th percentile
            } else {
                (50 - ((excess_ratio - 100) / 2)) as u8
            }
        }
    }
}

// =============================================================================
// CLAIMS CONFIGURATION
// =============================================================================

/// Claims configuration
/// PDA seeds: ["claims_config"]
#[account]
#[derive(InitSpace)]
pub struct ClaimsConfig {
    /// Authority (DAO)
    pub authority: Pubkey,

    /// Governance program for authorization
    pub governance_program: Pubkey,

    /// Reserves program for payouts
    pub reserves_program: Pubkey,

    /// Claims Committee multisig address
    pub claims_committee: Pubkey,

    /// Total claims submitted
    pub total_claims_submitted: u64,

    /// Total claims approved
    pub total_claims_approved: u64,

    /// Total claims denied
    pub total_claims_denied: u64,

    /// Total USDC paid out
    pub total_paid_out: u64,

    /// Auto-approve threshold (claims below this may be auto-approved)
    pub auto_approve_threshold: u64,

    /// Shock claim threshold (claims above this require DAO vote)
    pub shock_claim_threshold: u64,

    /// Required attestations for approval
    pub required_attestations: u8,

    /// Maximum attestation time (seconds)
    pub max_attestation_time: i64,

    /// Is claims processing active
    pub is_active: bool,

    /// Bump seed
    pub bump: u8,
}

impl ClaimsConfig {
    pub const SEED_PREFIX: &'static [u8] = b"claims_config";

    // =========================================================================
    // AUTO-APPROVE THRESHOLDS (Fast-Lane)
    // =========================================================================
    
    /// Standard scale defaults (10,000+ members)
    pub const DEFAULT_AUTO_APPROVE: u64 = 1_000_000_000; // $1,000
    
    /// Bootstrap scale defaults (< 1,000 members)
    /// More conservative for small pool variance
    pub const BOOTSTRAP_AUTO_APPROVE: u64 = 500_000_000; // $500
    
    /// Fast-lane limits (claims per member per month)
    pub const DEFAULT_FAST_LANE_LIMIT: u8 = 5;
    
    // =========================================================================
    // SHOCK CLAIM THRESHOLDS (Scale-Dependent)
    // =========================================================================
    //
    // CRITICAL: Shock threshold must scale with pool size/reserves.
    // Fixed $100k at $1.5M capital = 6.7% of reserves (too aggressive)
    // 
    // Two approaches available:
    // 1. Member-count based (simple, used for bootstrapping)
    // 2. Percentage-of-reserves based (preferred, more accurate)
    //
    // Shock claims require DAO vote due to potential pool impact
    
    /// Member-count thresholds (backwards compatible)
    pub const DEFAULT_SHOCK_THRESHOLD: u64 = 100_000_000_000; // $100,000 (large pool)
    pub const BOOTSTRAP_SHOCK_THRESHOLD: u64 = 25_000_000_000; // $25,000 (small pool)
    
    /// Percentage-based shock threshold (preferred method)
    /// Default: 5% of total reserves triggers DAO vote
    pub const SHOCK_THRESHOLD_BPS: u16 = 500; // 5%
    
    /// Minimum shock threshold (floor) - even at very small reserves
    pub const SHOCK_THRESHOLD_MIN: u64 = 10_000_000_000; // $10,000
    
    /// Maximum shock threshold (ceiling) - prevents runaway at large scale
    pub const SHOCK_THRESHOLD_MAX: u64 = 100_000_000_000; // $100,000
    
    // =========================================================================
    // ATTESTATION CONFIGURATION
    // =========================================================================
    
    pub const DEFAULT_REQUIRED_ATTESTATIONS: u8 = 2;
    pub const DEFAULT_MAX_ATTESTATION_TIME: i64 = 48 * 60 * 60; // 48 hours
    
    // =========================================================================
    // HELPER FUNCTIONS
    // =========================================================================
    
    /// Get auto-approve threshold based on member count
    pub fn get_auto_approve_threshold(member_count: u32) -> u64 {
        if member_count < 1000 {
            Self::BOOTSTRAP_AUTO_APPROVE
        } else {
            Self::DEFAULT_AUTO_APPROVE
        }
    }
    
    /// Get shock threshold based on member count (simple method)
    pub fn get_shock_threshold(member_count: u32) -> u64 {
        if member_count < 1000 {
            Self::BOOTSTRAP_SHOCK_THRESHOLD
        } else {
            Self::DEFAULT_SHOCK_THRESHOLD
        }
    }
    
    /// Calculate dynamic shock threshold based on total reserves (preferred)
    /// This scales appropriately for both small ($1.5M) and large ($50M+) pools
    /// 
    /// # Arguments
    /// * `total_reserves` - Total USDC in all reserve tiers (6 decimals)
    /// 
    /// # Returns
    /// * Shock threshold in USDC lamports, clamped between MIN and MAX
    /// 
    /// # Examples
    /// * $1.5M reserves → 5% = $75,000 (within bounds)
    /// * $500k reserves → 5% = $25,000 (within bounds)  
    /// * $100k reserves → 5% = $5,000 → clamped to MIN = $10,000
    /// * $50M reserves → 5% = $2.5M → clamped to MAX = $100,000
    pub fn calculate_shock_threshold_from_reserves(total_reserves: u64) -> u64 {
        let threshold = total_reserves
            .saturating_mul(Self::SHOCK_THRESHOLD_BPS as u64)
            .checked_div(10000)
            .unwrap_or(Self::SHOCK_THRESHOLD_MIN);
        
        // Clamp between min and max
        threshold.clamp(Self::SHOCK_THRESHOLD_MIN, Self::SHOCK_THRESHOLD_MAX)
    }
    
    /// Determine the most appropriate shock threshold given all available data
    /// Prefers reserve-based calculation when reserves are known
    pub fn get_effective_shock_threshold(
        member_count: u32,
        total_reserves: Option<u64>,
    ) -> u64 {
        match total_reserves {
            Some(reserves) if reserves > 0 => {
                Self::calculate_shock_threshold_from_reserves(reserves)
            }
            _ => Self::get_shock_threshold(member_count),
        }
    }
}

/// Benefit schedule defining coverage limits
/// PDA seeds: ["benefit_schedule"]
#[account]
#[derive(InitSpace)]
pub struct BenefitSchedule {
    /// Schedule name/identifier
    #[max_len(32)]
    pub name: String,

    /// Individual annual maximum (USDC)
    pub individual_annual_max: u64,

    /// Family annual maximum (USDC)
    pub family_annual_max: u64,

    /// Per-incident maximum (USDC)
    pub per_incident_max: u64,

    /// Deductible per individual (USDC)
    pub individual_deductible: u64,

    /// Deductible per family (USDC)
    pub family_deductible: u64,

    /// Coinsurance percentage (basis points, 8000 = 80% coverage)
    pub coinsurance_bps: u16,

    /// Out-of-pocket maximum individual (USDC)
    pub oop_max_individual: u64,

    /// Out-of-pocket maximum family (USDC)
    pub oop_max_family: u64,

    /// Waiting period for pre-existing conditions (days)
    pub preexisting_waiting_days: u16,

    /// Is this schedule active
    pub is_active: bool,

    /// Benefit categories with specific limits
    #[max_len(20)]
    pub category_limits: Vec<CategoryLimit>,

    /// Last updated
    pub last_updated: i64,

    /// Bump seed
    pub bump: u8,
}

impl BenefitSchedule {
    pub const SEED_PREFIX: &'static [u8] = b"benefit_schedule";
}

/// Category-specific benefit limit
#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct CategoryLimit {
    /// Category type
    pub category: ClaimCategory,
    /// Annual limit for this category (USDC)
    pub annual_limit: u64,
    /// Per-visit/incident limit (USDC)
    pub per_visit_limit: u64,
    /// Coinsurance override (0 = use default)
    pub coinsurance_override_bps: u16,
}

/// Claim categories
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum ClaimCategory {
    Emergency,
    Hospitalization,
    Surgery,
    OutpatientCare,
    PrimaryCare,
    SpecialistVisit,
    DiagnosticImaging,
    Laboratory,
    Prescription,
    MentalHealth,
    Maternity,
    Preventive,
    Rehabilitation,
    DurableMedicalEquipment,
    Other,
}

/// Individual claim account
/// PDA seeds: ["claim", claim_id]
#[account]
#[derive(InitSpace)]
pub struct ClaimAccount {
    /// Unique claim ID
    pub claim_id: u64,

    /// Member who submitted the claim
    pub member: Pubkey,

    /// Provider (if known)
    pub provider: Option<Pubkey>,

    /// Claim category
    pub category: ClaimCategory,

    /// Requested amount (USDC)
    pub requested_amount: u64,

    /// Approved amount (may differ from requested)
    pub approved_amount: u64,

    /// Amount already paid
    pub paid_amount: u64,

    /// Claim status
    pub status: ClaimStatus,

    /// Submission timestamp
    pub submitted_at: i64,

    /// Last status change timestamp
    pub status_changed_at: i64,

    /// Service date
    pub service_date: i64,

    /// Description/notes hash (IPFS or on-chain ref)
    #[max_len(64)]
    pub description_hash: String,

    /// Number of attestations received
    pub attestation_count: u8,

    /// Denial reason (if denied)
    #[max_len(128)]
    pub denial_reason: String,

    /// Is this a shock claim (exceeds threshold)
    pub is_shock_claim: bool,

    /// Bump seed
    pub bump: u8,
}

impl ClaimAccount {
    pub const SEED_PREFIX: &'static [u8] = b"claim";
}

/// Claim status state machine
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum ClaimStatus {
    /// Just submitted, awaiting review
    Submitted,
    /// Under AI/automated review
    UnderReview,
    /// Awaiting committee attestations
    PendingAttestation,
    /// Awaiting DAO vote (shock claim)
    PendingDaoVote,
    /// Approved, awaiting payment
    Approved,
    /// Payment in progress
    PaymentPending,
    /// Fully paid
    Paid,
    /// Denied
    Denied,
    /// Appealed
    Appealed,
    /// Closed (after appeal or finalization)
    Closed,
    /// Cancelled by member
    Cancelled,
}

impl Default for ClaimStatus {
    fn default() -> Self {
        ClaimStatus::Submitted
    }
}

/// Attestor registry - tracks authorized claim reviewers
/// PDA seeds: ["attestor_registry"]
#[account]
#[derive(InitSpace)]
pub struct AttestorRegistry {
    /// List of authorized attestors (Claims Committee members)
    #[max_len(20)]
    pub attestors: Vec<Pubkey>,

    /// Attestor count
    pub attestor_count: u8,

    /// Total attestations made
    pub total_attestations: u64,

    /// Bump seed
    pub bump: u8,
}

impl AttestorRegistry {
    pub const SEED_PREFIX: &'static [u8] = b"attestor_registry";
    pub const MAX_ATTESTORS: usize = 20;

    pub fn is_attestor(&self, pubkey: &Pubkey) -> bool {
        self.attestors.contains(pubkey)
    }
}

/// Attestation record for a specific claim
/// PDA seeds: ["attestation", claim_id, attestor]
#[account]
#[derive(InitSpace)]
pub struct Attestation {
    /// Claim being attested
    pub claim_id: u64,

    /// Attestor who made this attestation
    pub attestor: Pubkey,

    /// Recommendation
    pub recommendation: AttestationRecommendation,

    /// Recommended amount (may differ from requested)
    pub recommended_amount: u64,

    /// Notes/reasoning hash
    #[max_len(64)]
    pub notes_hash: String,

    /// Timestamp
    pub attested_at: i64,

    /// Bump seed
    pub bump: u8,
}

impl Attestation {
    pub const SEED_PREFIX: &'static [u8] = b"attestation";
}

/// Attestation recommendation
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum AttestationRecommendation {
    /// Approve in full
    ApproveFull,
    /// Approve partial amount
    ApprovePartial,
    /// Deny
    Deny,
    /// Need more information
    NeedMoreInfo,
    /// Escalate to DAO
    EscalateToDao,
}

// =============================================================================
// AI/ML CLAIMS ORACLE INFRASTRUCTURE
// Supports off-chain AI processing with on-chain decision recording
// =============================================================================

/// Claims Oracle configuration
/// PDA seeds: ["claims_oracle"]
#[account]
#[derive(InitSpace)]
pub struct ClaimsOracle {
    /// Authority for oracle management
    pub authority: Pubkey,
    
    /// Authorized oracle signers (AI service endpoints)
    #[max_len(5)]
    pub authorized_signers: Vec<Pubkey>,
    
    /// Required signatures for AI decision to be valid
    pub required_sigs: u8,
    
    /// Total decisions processed
    pub total_decisions: u64,
    
    /// Decisions auto-approved by AI
    pub auto_approved: u64,
    
    /// Decisions auto-denied by AI
    pub auto_denied: u64,
    
    /// Decisions escalated to committee
    pub escalated: u64,
    
    /// Decisions overturned by committee (measures AI accuracy)
    pub overturned: u64,
    
    /// Oracle accuracy rate (bps) - (total - overturned) / total
    pub accuracy_rate_bps: u16,
    
    /// Is oracle active
    pub is_active: bool,
    
    /// Minimum confidence for auto-approve (bps, e.g., 9500 = 95%)
    pub auto_approve_confidence_bps: u16,
    
    /// Maximum fraud score for auto-approve (bps, e.g., 3000 = 30%)
    pub max_fraud_score_bps: u16,
    
    /// Minimum confidence for auto-deny (bps)
    pub auto_deny_confidence_bps: u16,
    
    /// Bump seed
    pub bump: u8,
}

impl ClaimsOracle {
    pub const SEED_PREFIX: &'static [u8] = b"claims_oracle";
    
    // Default confidence thresholds
    pub const DEFAULT_AUTO_APPROVE_CONFIDENCE: u16 = 9500; // 95%
    pub const DEFAULT_MAX_FRAUD_SCORE: u16 = 3000; // 30%
    pub const DEFAULT_AUTO_DENY_CONFIDENCE: u16 = 9500; // 95% confident it's fraud
    
    /// Check if a signer is authorized
    pub fn is_authorized(&self, signer: &Pubkey) -> bool {
        self.authorized_signers.contains(signer)
    }
    
    /// Update accuracy rate
    pub fn update_accuracy(&mut self) {
        if self.total_decisions == 0 {
            self.accuracy_rate_bps = 0;
            return;
        }
        
        let accurate = self.total_decisions.saturating_sub(self.overturned);
        self.accuracy_rate_bps = (accurate
            .saturating_mul(10000)
            .checked_div(self.total_decisions)
            .unwrap_or(0) as u16)
            .min(10000);
    }
}

/// AI decision record for a specific claim
/// PDA seeds: ["ai_decision", claim_id]
#[account]
#[derive(InitSpace)]
pub struct AiDecisionRecord {
    /// Claim ID this decision applies to
    pub claim_id: u64,
    
    /// Decision type
    pub decision: AiDecisionType,
    
    /// Overall confidence score (bps)
    pub confidence_bps: u16,
    
    /// Price reasonableness score (bps, higher = more reasonable)
    pub price_score_bps: u16,
    
    /// Fraud detection score (bps, higher = more suspicious)
    pub fraud_score_bps: u16,
    
    /// Consistency score (diagnosis-procedure match)
    pub consistency_score_bps: u16,
    
    /// Flags raised during analysis
    #[max_len(5)]
    pub flags: Vec<AiFlag>,
    
    /// Suggested approved amount (may differ from requested)
    pub suggested_amount: u64,
    
    /// Reference price from UCR database
    pub reference_price: u64,
    
    /// Oracle signer who submitted this decision
    pub submitted_by: Pubkey,
    
    /// Timestamp of decision
    pub decided_at: i64,
    
    /// Was this decision overturned by committee
    pub overturned: bool,
    
    /// Bump seed
    pub bump: u8,
}

impl AiDecisionRecord {
    pub const SEED_PREFIX: &'static [u8] = b"ai_decision";
}

/// AI decision type
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum AiDecisionType {
    /// Auto-approve - high confidence, low fraud risk
    AutoApprove,
    /// Auto-deny - high confidence fraud or policy violation
    AutoDeny,
    /// Escalate to committee review
    CommitteeReview,
    /// Need more information from member
    RequestInfo,
}

/// Flags raised by AI analysis
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum AiFlag {
    /// Price significantly above UCR average
    HighPrice,
    /// Price significantly below UCR average (may indicate unbundling)
    LowPrice,
    /// Diagnosis doesn't match procedure code
    DiagnosisMismatch,
    /// High claim frequency from this member
    HighFrequency,
    /// Provider has elevated denial rate
    ProviderRisk,
    /// Missing required documentation
    IncompleteDoc,
    /// Duplicate claim detected
    PossibleDuplicate,
    /// Service during waiting period
    WaitingPeriod,
    /// Benefit limit exceeded
    LimitExceeded,
    /// New member (< 30 days enrolled)
    NewMember,
    /// Experimental/investigational treatment
    Experimental,
    /// Out-of-network provider
    OutOfNetwork,
}

// =============================================================================
// FAST-LANE TRACKING
// Prevents abuse of auto-approval system
// =============================================================================

/// Fast-lane claim tracker for a member
/// PDA seeds: ["fast_lane", member]
#[account]
#[derive(InitSpace)]
pub struct FastLaneTracker {
    /// Member this tracker belongs to
    pub member: Pubkey,
    
    /// Fast-lane claims this period
    pub claims_this_period: u8,
    
    /// Period start timestamp
    pub period_start: i64,
    
    /// Total fast-lane claims all-time
    pub total_fast_lane_claims: u64,
    
    /// Total fast-lane amount all-time
    pub total_fast_lane_amount: u64,
    
    /// Last fast-lane claim timestamp
    pub last_claim_at: i64,
    
    /// Is member flagged for review (exceeded limits)
    pub flagged: bool,
    
    /// Bump seed
    pub bump: u8,
}

impl FastLaneTracker {
    pub const SEED_PREFIX: &'static [u8] = b"fast_lane";
    
    /// Period length (30 days in seconds)
    pub const PERIOD_LENGTH: i64 = 30 * 24 * 60 * 60;
    
    /// Check if member can use fast-lane
    pub fn can_use_fast_lane(&self, current_time: i64, limit: u8) -> bool {
        if self.flagged {
            return false;
        }
        
        // Check if we're in a new period
        if current_time - self.period_start >= Self::PERIOD_LENGTH {
            return true; // New period, counter would reset
        }
        
        self.claims_this_period < limit
    }
    
    /// Record a fast-lane claim
    pub fn record_claim(&mut self, amount: u64, current_time: i64, limit: u8) {
        // Reset period if needed
        if current_time - self.period_start >= Self::PERIOD_LENGTH {
            self.period_start = current_time;
            self.claims_this_period = 0;
        }
        
        self.claims_this_period += 1;
        self.total_fast_lane_claims += 1;
        self.total_fast_lane_amount += amount;
        self.last_claim_at = current_time;
        
        // Flag if hitting limit frequently
        if self.claims_this_period >= limit {
            self.flagged = true;
        }
    }
}

// =============================================================================
// UCR (USUAL, CUSTOMARY, REASONABLE) PRICE REFERENCE
// For price reasonableness validation
// =============================================================================

/// Regional UCR price reference entry
/// PDA seeds: ["ucr", procedure_code, region_code]
#[account]
#[derive(InitSpace)]
pub struct UcrPriceEntry {
    /// Procedure code (CPT/HCPCS as u32 hash)
    pub procedure_code: u32,
    
    /// Region code
    pub region_code: u8,
    
    /// 25th percentile price (USDC)
    pub p25_price: u64,
    
    /// 50th percentile price (median, USDC)
    pub p50_price: u64,
    
    /// 75th percentile price (USDC)
    pub p75_price: u64,
    
    /// 90th percentile price (USDC)
    pub p90_price: u64,
    
    /// Sample size used for calculation
    pub sample_size: u32,
    
    /// Last updated timestamp
    pub last_updated: i64,
    
    /// Data source (0 = CMS, 1 = FAIR Health, 2 = Apollo internal)
    pub data_source: u8,
    
    /// Bump seed
    pub bump: u8,
}

impl UcrPriceEntry {
    pub const SEED_PREFIX: &'static [u8] = b"ucr";
    
    /// Check if a price is reasonable (within acceptable range)
    /// Returns: (is_reasonable, deviation_bps)
    pub fn check_price(&self, billed_amount: u64) -> (bool, u16) {
        if self.p50_price == 0 {
            return (true, 0); // No data, assume reasonable
        }
        
        // Calculate deviation from median
        let deviation = if billed_amount > self.p50_price {
            billed_amount.saturating_sub(self.p50_price)
                .saturating_mul(10000)
                .checked_div(self.p50_price)
                .unwrap_or(0)
        } else {
            self.p50_price.saturating_sub(billed_amount)
                .saturating_mul(10000)
                .checked_div(self.p50_price)
                .unwrap_or(0)
        };
        
        let deviation_bps = deviation.min(u16::MAX as u64) as u16;
        
        // Reasonable if within 200% of median (allowing for geographic variation)
        let is_reasonable = billed_amount <= self.p90_price.saturating_mul(2);
        
        (is_reasonable, deviation_bps)
    }
    
    /// Get suggested fair price for a claim
    pub fn suggested_price(&self) -> u64 {
        // Use 75th percentile as "fair" price
        self.p75_price
    }
}

// =============================================================================
// UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // SHOCK THRESHOLD TESTS
    // =========================================================================

    #[test]
    fn test_shock_threshold_from_reserves_small_pool() {
        // $1.5M reserves → 5% = $75,000 (within bounds)
        let reserves = 1_500_000_000_000u64; // $1.5M
        let threshold = ClaimsConfig::calculate_shock_threshold_from_reserves(reserves);
        
        // 5% of $1.5M = $75,000
        assert_eq!(threshold, 75_000_000_000);
    }

    #[test]
    fn test_shock_threshold_from_reserves_very_small() {
        // $100k reserves → 5% = $5,000 → clamped to MIN = $10,000
        let reserves = 100_000_000_000u64; // $100k
        let threshold = ClaimsConfig::calculate_shock_threshold_from_reserves(reserves);
        
        // Clamped to minimum
        assert_eq!(threshold, ClaimsConfig::SHOCK_THRESHOLD_MIN);
        assert_eq!(threshold, 10_000_000_000);
    }

    #[test]
    fn test_shock_threshold_from_reserves_large_pool() {
        // $50M reserves → 5% = $2.5M → clamped to MAX = $100,000
        let reserves = 50_000_000_000_000u64; // $50M
        let threshold = ClaimsConfig::calculate_shock_threshold_from_reserves(reserves);
        
        // Clamped to maximum
        assert_eq!(threshold, ClaimsConfig::SHOCK_THRESHOLD_MAX);
        assert_eq!(threshold, 100_000_000_000);
    }

    #[test]
    fn test_shock_threshold_from_reserves_medium_pool() {
        // $5M reserves → 5% = $250,000 → clamped to MAX = $100,000
        let reserves = 5_000_000_000_000u64; // $5M
        let threshold = ClaimsConfig::calculate_shock_threshold_from_reserves(reserves);
        
        // 5% of $5M = $250k, but clamped to $100k max
        assert_eq!(threshold, 100_000_000_000);
    }

    #[test]
    fn test_shock_threshold_zero_reserves() {
        // Zero reserves → minimum threshold
        let reserves = 0u64;
        let threshold = ClaimsConfig::calculate_shock_threshold_from_reserves(reserves);
        
        assert_eq!(threshold, ClaimsConfig::SHOCK_THRESHOLD_MIN);
    }

    #[test]
    fn test_shock_threshold_member_based_small() {
        // Small pool (< 1000 members)
        let threshold = ClaimsConfig::get_shock_threshold(500);
        assert_eq!(threshold, ClaimsConfig::BOOTSTRAP_SHOCK_THRESHOLD);
        assert_eq!(threshold, 25_000_000_000); // $25k
    }

    #[test]
    fn test_shock_threshold_member_based_large() {
        // Large pool (>= 1000 members)
        let threshold = ClaimsConfig::get_shock_threshold(2000);
        assert_eq!(threshold, ClaimsConfig::DEFAULT_SHOCK_THRESHOLD);
        assert_eq!(threshold, 100_000_000_000); // $100k
    }

    #[test]
    fn test_effective_shock_threshold_prefers_reserves() {
        // When reserves are known, prefer reserve-based calculation
        let threshold = ClaimsConfig::get_effective_shock_threshold(
            500, // member count
            Some(1_500_000_000_000), // $1.5M reserves
        );
        
        // Should use reserve-based calculation: 5% of $1.5M = $75k
        assert_eq!(threshold, 75_000_000_000);
    }

    #[test]
    fn test_effective_shock_threshold_fallback_to_members() {
        // When reserves unknown, use member-based calculation
        let threshold = ClaimsConfig::get_effective_shock_threshold(
            500, // member count
            None, // no reserves known
        );
        
        // Should use member-based: small pool = $25k
        assert_eq!(threshold, 25_000_000_000);
    }

    // =========================================================================
    // AUTO-APPROVE THRESHOLD TESTS
    // =========================================================================

    #[test]
    fn test_auto_approve_small_pool() {
        let threshold = ClaimsConfig::get_auto_approve_threshold(500);
        assert_eq!(threshold, ClaimsConfig::BOOTSTRAP_AUTO_APPROVE);
        assert_eq!(threshold, 500_000_000); // $500
    }

    #[test]
    fn test_auto_approve_large_pool() {
        let threshold = ClaimsConfig::get_auto_approve_threshold(2000);
        assert_eq!(threshold, ClaimsConfig::DEFAULT_AUTO_APPROVE);
        assert_eq!(threshold, 1_000_000_000); // $1,000
    }

    #[test]
    fn test_auto_approve_at_threshold() {
        // At exactly 1000 members, should use large pool threshold
        let threshold = ClaimsConfig::get_auto_approve_threshold(1000);
        assert_eq!(threshold, ClaimsConfig::DEFAULT_AUTO_APPROVE);
    }

    // =========================================================================
    // CONSTANTS VALIDATION TESTS
    // =========================================================================

    #[test]
    fn test_constants_relationships() {
        // MIN should be less than MAX
        assert!(ClaimsConfig::SHOCK_THRESHOLD_MIN < ClaimsConfig::SHOCK_THRESHOLD_MAX);
        
        // BPS should be reasonable (5%)
        assert_eq!(ClaimsConfig::SHOCK_THRESHOLD_BPS, 500);
        
        // Bootstrap thresholds should be more conservative (lower)
        assert!(ClaimsConfig::BOOTSTRAP_AUTO_APPROVE < ClaimsConfig::DEFAULT_AUTO_APPROVE);
        assert!(ClaimsConfig::BOOTSTRAP_SHOCK_THRESHOLD < ClaimsConfig::DEFAULT_SHOCK_THRESHOLD);
    }

    #[test]
    fn test_fast_lane_limit() {
        assert_eq!(ClaimsConfig::DEFAULT_FAST_LANE_LIMIT, 5);
    }

    #[test]
    fn test_attestation_defaults() {
        assert_eq!(ClaimsConfig::DEFAULT_REQUIRED_ATTESTATIONS, 2);
        assert_eq!(ClaimsConfig::DEFAULT_MAX_ATTESTATION_TIME, 48 * 60 * 60);
    }

    // =========================================================================
    // CLAIM STATUS TESTS
    // =========================================================================

    #[test]
    fn test_claim_status_default() {
        assert_eq!(ClaimStatus::default(), ClaimStatus::Submitted);
    }

    #[test]
    fn test_claim_category_coverage() {
        // Ensure all categories are distinct
        assert_ne!(ClaimCategory::Emergency, ClaimCategory::Hospitalization);
        assert_ne!(ClaimCategory::PrimaryCare, ClaimCategory::SpecialistVisit);
    }

    #[test]
    fn test_attestation_recommendation_equality() {
        assert_eq!(AttestationRecommendation::ApproveFull, AttestationRecommendation::ApproveFull);
        assert_ne!(AttestationRecommendation::ApproveFull, AttestationRecommendation::Deny);
    }
}
