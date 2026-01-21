use anchor_lang::prelude::*;

#[error_code]
pub enum ReinsuranceError {
    // === Authorization Errors ===
    
    #[msg("Unauthorized: caller is not the authority")]
    Unauthorized,
    
    #[msg("Unauthorized: caller is not on the reinsurance committee")]
    NotCommitteeMember,
    
    // === Treaty Errors ===
    
    #[msg("Treaty is not active")]
    TreatyNotActive,
    
    #[msg("Treaty has expired")]
    TreatyExpired,
    
    #[msg("Treaty is suspended")]
    TreatySuspended,
    
    #[msg("Treaty effective date must be before expiration")]
    InvalidTreatyDates,
    
    #[msg("Treaty already exists with this ID")]
    TreatyAlreadyExists,
    
    #[msg("Treaty not found")]
    TreatyNotFound,
    
    #[msg("Cannot modify treaty in current status")]
    TreatyCannotBeModified,
    
    #[msg("Treaty type mismatch for operation")]
    TreatyTypeMismatch,
    
    #[msg("Maximum number of treaties reached")]
    MaxTreatiesReached,
    
    // === Recovery Claim Errors ===
    
    #[msg("Recovery claim already exists")]
    RecoveryClaimExists,
    
    #[msg("Recovery claim not found")]
    RecoveryClaimNotFound,
    
    #[msg("Recovery claim already settled")]
    RecoveryAlreadySettled,
    
    #[msg("Recovery claim is not in valid state for this operation")]
    InvalidRecoveryStatus,
    
    #[msg("Recovery amount exceeds claimed amount")]
    RecoveryExceedsClaim,
    
    #[msg("Amount does not exceed attachment point")]
    BelowAttachmentPoint,
    
    #[msg("No excess amount to claim")]
    NoExcessAmount,
    
    #[msg("Claim has not been submitted to reinsurer yet")]
    ClaimNotSubmitted,
    
    // === Stop-Loss Errors ===
    
    #[msg("Specific stop-loss not triggered for this member")]
    StopLossNotTriggered,
    
    #[msg("Aggregate stop-loss not triggered")]
    AggregateNotTriggered,
    
    #[msg("Catastrophic layer not triggered")]
    CatastrophicNotTriggered,
    
    #[msg("Member accumulator not found")]
    AccumulatorNotFound,
    
    #[msg("Member has no claims this policy year")]
    NoClaimsForMember,
    
    // === Financial Errors ===
    
    #[msg("Invalid attachment point: must be greater than zero")]
    InvalidAttachmentPoint,
    
    #[msg("Invalid coinsurance rate: must be between 0 and 10000 bps")]
    InvalidCoinsuranceRate,
    
    #[msg("Invalid trigger ratio")]
    InvalidTriggerRatio,
    
    #[msg("Premium not fully paid")]
    PremiumNotPaid,
    
    #[msg("Insufficient premium budget")]
    InsufficientPremiumBudget,
    
    #[msg("Invalid premium amount")]
    InvalidPremiumAmount,
    
    #[msg("Coverage limit exceeded")]
    CoverageLimitExceeded,
    
    // === Policy Year Errors ===
    
    #[msg("Policy year has not started")]
    PolicyYearNotStarted,
    
    #[msg("Policy year has ended")]
    PolicyYearEnded,
    
    #[msg("Invalid policy year dates")]
    InvalidPolicyYearDates,
    
    #[msg("Cannot reset counters during active policy year")]
    CannotResetDuringActiveYear,
    
    // === Configuration Errors ===
    
    #[msg("Invalid configuration parameter")]
    InvalidConfiguration,
    
    #[msg("Expected claims must be set before operations")]
    ExpectedClaimsNotSet,
    
    #[msg("Aggregate trigger ratio must be greater than 10000 (100%)")]
    InvalidAggregateTrigger,
    
    #[msg("Catastrophic trigger must be greater than aggregate trigger")]
    InvalidCatastrophicTrigger,
    
    #[msg("Ceiling ratio must be greater than trigger ratio")]
    InvalidCeilingRatio,
    
    // === Numeric Errors ===
    
    #[msg("Arithmetic overflow")]
    Overflow,
    
    #[msg("Arithmetic underflow")]
    Underflow,
    
    #[msg("Division by zero")]
    DivisionByZero,
    
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    
    // === Integration Errors ===
    
    #[msg("Invalid claims program reference")]
    InvalidClaimsProgram,
    
    #[msg("Claims data verification failed")]
    ClaimsDataMismatch,
    
    #[msg("Cross-program invocation failed")]
    CpiError,
}
