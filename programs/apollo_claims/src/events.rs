// programs/apollo_claims/src/events.rs

use crate::state::{AttestationRecommendation, ClaimCategory, ClaimStatus};
use anchor_lang::prelude::*;

/// Emitted when claims config is initialized
#[event]
pub struct ClaimsConfigInitialized {
    pub authority: Pubkey,
    pub claims_committee: Pubkey,
    pub auto_approve_threshold: u64,
    pub shock_claim_threshold: u64,
    pub timestamp: i64,
}

/// Emitted when benefit schedule is set/updated
#[event]
pub struct BenefitScheduleUpdated {
    pub name: String,
    pub individual_annual_max: u64,
    pub family_annual_max: u64,
    pub updater: Pubkey,
    pub timestamp: i64,
}

/// Emitted when a claim is submitted
#[event]
pub struct ClaimSubmitted {
    pub claim_id: u64,
    pub member: Pubkey,
    pub category: ClaimCategory,
    pub requested_amount: u64,
    pub is_shock_claim: bool,
    pub timestamp: i64,
}

/// Emitted when a claim is attested
#[event]
pub struct ClaimAttested {
    pub claim_id: u64,
    pub attestor: Pubkey,
    pub recommendation: AttestationRecommendation,
    pub recommended_amount: u64,
    pub attestation_count: u8,
    pub timestamp: i64,
}

/// Emitted when a claim is approved
#[event]
pub struct ClaimApproved {
    pub claim_id: u64,
    pub member: Pubkey,
    pub requested_amount: u64,
    pub approved_amount: u64,
    pub approver: Pubkey,
    pub timestamp: i64,
}

/// Emitted when a claim is denied
#[event]
pub struct ClaimDenied {
    pub claim_id: u64,
    pub member: Pubkey,
    pub requested_amount: u64,
    pub reason: String,
    pub denier: Pubkey,
    pub timestamp: i64,
}

/// Emitted when a claim is paid
#[event]
pub struct ClaimPaid {
    pub claim_id: u64,
    pub member: Pubkey,
    pub amount: u64,
    pub recipient: Pubkey,
    pub timestamp: i64,
}

/// Emitted when a claim is closed
#[event]
pub struct ClaimClosed {
    pub claim_id: u64,
    pub final_status: ClaimStatus,
    pub total_paid: u64,
    pub timestamp: i64,
}

/// Emitted when a claim is cancelled
#[event]
pub struct ClaimCancelled {
    pub claim_id: u64,
    pub member: Pubkey,
    pub timestamp: i64,
}

/// Emitted when a claim is appealed
#[event]
pub struct ClaimAppealed {
    pub claim_id: u64,
    pub member: Pubkey,
    pub previous_status: ClaimStatus,
    pub timestamp: i64,
}

/// Emitted when an attestor is added
#[event]
pub struct AttestorAdded {
    pub attestor: Pubkey,
    pub total_attestors: u8,
    pub timestamp: i64,
}

/// Emitted when an attestor is removed
#[event]
pub struct AttestorRemoved {
    pub attestor: Pubkey,
    pub total_attestors: u8,
    pub timestamp: i64,
}

/// Emitted when claim status changes
#[event]
pub struct ClaimStatusChanged {
    pub claim_id: u64,
    pub old_status: ClaimStatus,
    pub new_status: ClaimStatus,
    pub timestamp: i64,
}

// =============================================================================
// FAST-LANE EVENTS
// =============================================================================

/// Emitted when a claim is auto-approved via fast-lane
#[event]
pub struct ClaimAutoApproved {
    pub claim_id: u64,
    pub member: Pubkey,
    pub amount: u64,
    pub category: ClaimCategory,
    pub processing_time_ms: u64,
    pub timestamp: i64,
}

/// Emitted when a member's fast-lane access is denied
#[event]
pub struct FastLaneDenied {
    pub member: Pubkey,
    pub reason: String,
    pub flagged_by: Pubkey,
    pub timestamp: i64,
}

/// Emitted when a member's fast-lane access is restored
#[event]
pub struct FastLaneRestored {
    pub member: Pubkey,
    pub restored_by: Pubkey,
    pub timestamp: i64,
}

// =============================================================================
// AI/ML PROCESSING EVENTS
// =============================================================================

/// Emitted when AI submits a decision for a claim
// AiDecisionSubmitted, AiDecisionOverturned, and AiDecisionRecorded
// are defined in ai_oracle.rs (canonical) to avoid duplicate discriminators

// =============================================================================
// PHASE TRANSITION EVENTS
// =============================================================================

/// Emitted when protocol phase changes
#[event]
pub struct PhaseTransitioned {
    pub old_phase: u8,
    pub new_phase: u8,
    pub transitioned_by: Pubkey,
    pub timestamp: i64,
}

/// Emitted when phase requirements are updated
#[event]
pub struct PhaseRequirementsUpdated {
    pub phase: u8,
    pub updated_by: Pubkey,
    pub timestamp: i64,
}

// =============================================================================
// REINSURANCE EVENTS
// =============================================================================

/// Emitted when reinsurance recovery is triggered
#[event]
pub struct ReinsuranceTriggered {
    pub claim_id: u64,
    pub claim_amount: u64,
    pub recovery_type: u8, // 0 = specific, 1 = aggregate
    pub recovery_amount: u64,
    pub timestamp: i64,
}

/// Emitted when reinsurance policy is renewed
#[event]
pub struct ReinsurancePolicyRenewed {
    pub policy_start: i64,
    pub policy_end: i64,
    pub premium_paid: u64,
    pub expected_claims: u64,
    pub timestamp: i64,
}
