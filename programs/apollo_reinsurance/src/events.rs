use anchor_lang::prelude::*;

use crate::state::{ReinsuranceLayerType, TreatyStatus};

// ============================================================================
// CONFIGURATION EVENTS
// ============================================================================

#[event]
pub struct ReinsuranceConfigInitialized {
    pub authority: Pubkey,
    pub reinsurance_committee: Pubkey,
    pub policy_year_start: i64,
    pub policy_year_end: i64,
    pub expected_annual_claims: u64,
    pub timestamp: i64,
}

#[event]
pub struct PolicyYearUpdated {
    pub old_year_start: i64,
    pub old_year_end: i64,
    pub new_year_start: i64,
    pub new_year_end: i64,
    pub expected_annual_claims: u64,
    pub ytd_claims_reset: bool,
    pub timestamp: i64,
}

#[event]
pub struct ExpectedClaimsUpdated {
    pub old_expected: u64,
    pub new_expected: u64,
    pub updater: Pubkey,
    pub reason: [u8; 32], // hashed reason
    pub timestamp: i64,
}

#[event]
pub struct TriggerRatiosUpdated {
    pub old_aggregate_bps: u16,
    pub new_aggregate_bps: u16,
    pub old_catastrophic_bps: u16,
    pub new_catastrophic_bps: u16,
    pub old_ceiling_bps: u16,
    pub new_ceiling_bps: u16,
    pub timestamp: i64,
}

// ============================================================================
// TREATY EVENTS
// ============================================================================

#[event]
pub struct TreatyCreated {
    pub treaty_id: u64,
    pub treaty_pubkey: Pubkey,
    pub layer_type: ReinsuranceLayerType,
    pub reinsurer_id: [u8; 32],
    pub attachment_point: u64,
    pub coinsurance_rate_bps: u16,
    pub coverage_limit: u64,
    pub annual_premium: u64,
    pub effective_date: i64,
    pub expiration_date: i64,
    pub timestamp: i64,
}

#[event]
pub struct TreatyActivated {
    pub treaty_id: u64,
    pub treaty_pubkey: Pubkey,
    pub layer_type: ReinsuranceLayerType,
    pub activated_by: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct TreatyStatusChanged {
    pub treaty_id: u64,
    pub treaty_pubkey: Pubkey,
    pub old_status: TreatyStatus,
    pub new_status: TreatyStatus,
    pub changed_by: Pubkey,
    pub reason: [u8; 32],
    pub timestamp: i64,
}

#[event]
pub struct TreatyExpired {
    pub treaty_id: u64,
    pub treaty_pubkey: Pubkey,
    pub layer_type: ReinsuranceLayerType,
    pub total_claims_submitted: u64,
    pub total_recoveries_received: u64,
    pub timestamp: i64,
}

#[event]
pub struct TreatyRenewed {
    pub old_treaty_id: u64,
    pub new_treaty_id: u64,
    pub new_treaty_pubkey: Pubkey,
    pub layer_type: ReinsuranceLayerType,
    pub new_effective_date: i64,
    pub new_expiration_date: i64,
    pub new_annual_premium: u64,
    pub timestamp: i64,
}

#[event]
pub struct PremiumPaid {
    pub treaty_id: u64,
    pub treaty_pubkey: Pubkey,
    pub amount: u64,
    pub total_paid: u64,
    pub annual_premium: u64,
    pub payer: Pubkey,
    pub timestamp: i64,
}

// ============================================================================
// STOP-LOSS TRIGGER EVENTS
// ============================================================================

#[event]
pub struct SpecificStopLossTriggered {
    pub member: Pubkey,
    pub member_hash: [u8; 32],
    pub treaty_id: u64,
    pub total_claims: u64,
    pub attachment_point: u64,
    pub excess_amount: u64,
    pub apollo_portion: u64,
    pub reinsurer_portion: u64,
    pub triggering_claim_id: u64,
    pub timestamp: i64,
}

#[event]
pub struct AggregateStopLossTriggered {
    pub treaty_id: u64,
    pub ytd_claims: u64,
    pub expected_claims: u64,
    pub trigger_ratio_bps: u16,
    pub actual_ratio_bps: u64,
    pub excess_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct CatastrophicLayerTriggered {
    pub treaty_id: u64,
    pub ytd_claims: u64,
    pub expected_claims: u64,
    pub trigger_ratio_bps: u16,
    pub actual_ratio_bps: u64,
    pub timestamp: i64,
}

// ============================================================================
// RECOVERY CLAIM EVENTS
// ============================================================================

#[event]
pub struct RecoveryClaimFiled {
    pub claim_id: u64,
    pub claim_pubkey: Pubkey,
    pub treaty_id: u64,
    pub layer_type: ReinsuranceLayerType,
    pub total_claims_amount: u64,
    pub excess_amount: u64,
    pub claimed_amount: u64,
    pub filed_by: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct RecoveryClaimSubmitted {
    pub claim_id: u64,
    pub treaty_id: u64,
    pub claimed_amount: u64,
    pub documentation_hash: [u8; 32],
    pub submitted_by: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct RecoveryClaimApproved {
    pub claim_id: u64,
    pub treaty_id: u64,
    pub claimed_amount: u64,
    pub approved_amount: u64,
    pub reinsurer_reference: [u8; 32],
    pub timestamp: i64,
}

#[event]
pub struct RecoveryClaimDenied {
    pub claim_id: u64,
    pub treaty_id: u64,
    pub claimed_amount: u64,
    pub reason_hash: [u8; 32],
    pub timestamp: i64,
}

#[event]
pub struct RecoveryClaimDisputed {
    pub claim_id: u64,
    pub treaty_id: u64,
    pub claimed_amount: u64,
    pub disputed_amount: u64,
    pub reason_hash: [u8; 32],
    pub timestamp: i64,
}

#[event]
pub struct RecoverySettled {
    pub claim_id: u64,
    pub treaty_id: u64,
    pub claimed_amount: u64,
    pub approved_amount: u64,
    pub received_amount: u64,
    pub is_partial: bool,
    pub timestamp: i64,
}

// ============================================================================
// CLAIMS ACCUMULATOR EVENTS
// ============================================================================

#[event]
pub struct MemberAccumulatorCreated {
    pub member: Pubkey,
    pub policy_year: u16,
    pub timestamp: i64,
}

#[event]
pub struct MemberClaimRecorded {
    pub member: Pubkey,
    pub claim_amount: u64,
    pub ytd_total: u64,
    pub claims_count: u32,
    pub original_claim_id: u64,
    pub timestamp: i64,
}

#[event]
pub struct MemberStopLossBreached {
    pub member: Pubkey,
    pub ytd_claims: u64,
    pub attachment_point: u64,
    pub excess_amount: u64,
    pub breach_timestamp: i64,
}

// ============================================================================
// AGGREGATE TRACKING EVENTS
// ============================================================================

#[event]
pub struct MonthlyAggregateUpdated {
    pub policy_year: u16,
    pub month: u8,
    pub total_claims: u64,
    pub claims_count: u32,
    pub expected_claims: u64,
    pub ratio_bps: u16,
    pub ytd_through_month: u64,
    pub timestamp: i64,
}

#[event]
pub struct ClaimsThresholdWarning {
    pub current_ytd: u64,
    pub expected_annual: u64,
    pub current_ratio_bps: u64,
    pub warning_threshold_bps: u16,
    pub aggregate_trigger_bps: u16,
    pub remaining_headroom: u64,
    pub timestamp: i64,
}

// ============================================================================
// YEAR-END EVENTS
// ============================================================================

#[event]
pub struct YearEndReconciliation {
    pub policy_year_start: i64,
    pub policy_year_end: i64,
    pub total_claims_paid: u64,
    pub expected_claims: u64,
    pub total_recoveries_filed: u64,
    pub total_recoveries_received: u64,
    pub total_premium_paid: u64,
    pub aggregate_triggered: bool,
    pub catastrophic_triggered: bool,
    pub net_reinsurance_benefit: i64, // can be negative if premium > recoveries
    pub timestamp: i64,
}

#[event]
pub struct AccumulatorsReset {
    pub policy_year: u16,
    pub members_reset: u32,
    pub total_ytd_reset: u64,
    pub timestamp: i64,
}
