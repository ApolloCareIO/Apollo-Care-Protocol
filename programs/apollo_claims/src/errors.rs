// programs/apollo_claims/src/errors.rs

use anchor_lang::prelude::*;

#[error_code]
pub enum ClaimsError {
    #[msg("Claims processing not initialized")]
    NotInitialized,

    #[msg("Claims processing is paused")]
    ClaimsPaused,

    #[msg("Unauthorized: caller lacks permission")]
    Unauthorized,

    #[msg("Invalid claim status for this operation")]
    InvalidClaimStatus,

    #[msg("Claim not found")]
    ClaimNotFound,

    #[msg("Claim already processed")]
    ClaimAlreadyProcessed,

    #[msg("Invalid claim amount")]
    InvalidClaimAmount,

    #[msg("Claim amount exceeds benefit limit")]
    ExceedsBenefitLimit,

    #[msg("Member not eligible for this benefit")]
    MemberNotEligible,

    #[msg("Attestor not registered")]
    AttestorNotRegistered,

    #[msg("Attestor already attested this claim")]
    AlreadyAttested,

    #[msg("Insufficient attestations for approval")]
    InsufficientAttestations,

    #[msg("Attestation period expired")]
    AttestationExpired,

    #[msg("Shock claim requires DAO vote")]
    ShockClaimRequiresDaoVote,

    #[msg("Payment already completed")]
    AlreadyPaid,

    #[msg("Payment failed")]
    PaymentFailed,

    #[msg("Benefit schedule not active")]
    BenefitScheduleNotActive,

    #[msg("Invalid benefit schedule configuration")]
    InvalidBenefitSchedule,

    #[msg("Category limit exceeded")]
    CategoryLimitExceeded,

    #[msg("Annual limit exceeded")]
    AnnualLimitExceeded,

    #[msg("Deductible not met")]
    DeductibleNotMet,

    #[msg("Out of pocket maximum reached")]
    OopMaxReached,

    #[msg("Waiting period not complete")]
    WaitingPeriodNotComplete,

    #[msg("Invalid service date")]
    InvalidServiceDate,

    #[msg("Claim cannot be cancelled in current status")]
    CannotCancel,

    #[msg("Appeal not allowed for this claim")]
    AppealNotAllowed,

    #[msg("Maximum attestors reached")]
    MaxAttestorsReached,

    // AI Processing Errors
    #[msg("AI Oracle is inactive")]
    OracleInactive,

    #[msg("AI Oracle is disabled")]
    OracleDisabled,

    #[msg("Invalid oracle configuration")]
    InvalidConfiguration,

    #[msg("Insufficient oracle signers configured")]
    InsufficientSigners,

    #[msg("Claim exceeds fast-lane limit")]
    ExceedsFastLaneLimit,

    #[msg("Category not eligible for fast-lane")]
    CategoryNotEligible,

    #[msg("Monthly fast-lane limit exceeded")]
    FastLaneLimitExceeded,

    #[msg("Fast-lane processing is disabled")]
    FastLaneDisabled,

    #[msg("AI decision already recorded for this claim")]
    DecisionAlreadyRecorded,

    #[msg("Decision already overturned")]
    AlreadyOverturned,

    #[msg("Fraud detected - claim denied")]
    FraudDetected,

    #[msg("Member flagged for excessive fast-lane usage")]
    MemberFlagged,
}
