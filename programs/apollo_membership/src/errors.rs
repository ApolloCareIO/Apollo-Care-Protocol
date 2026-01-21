// programs/apollo_membership/src/errors.rs

use anchor_lang::prelude::*;

#[error_code]
pub enum MembershipError {
    #[msg("Membership system not initialized")]
    NotInitialized,

    #[msg("Unauthorized: caller lacks permission")]
    Unauthorized,

    #[msg("Enrollment is not currently open")]
    EnrollmentClosed,

    #[msg("Enrollment window is not active")]
    WindowNotActive,

    #[msg("Enrollment window has expired")]
    WindowExpired,

    #[msg("Enrollment cap reached for this window")]
    EnrollmentCapReached,

    #[msg("Member already enrolled")]
    AlreadyEnrolled,

    #[msg("Member not found")]
    MemberNotFound,

    #[msg("Invalid member age")]
    InvalidAge,

    #[msg("Invalid region code")]
    InvalidRegionCode,

    #[msg("Waiting period not complete")]
    WaitingPeriodNotComplete,

    #[msg("Coverage already active")]
    CoverageAlreadyActive,

    #[msg("Coverage not active")]
    CoverageNotActive,

    #[msg("Insufficient contribution amount")]
    InsufficientContribution,

    #[msg("Payment already made for this period")]
    AlreadyPaid,

    #[msg("Payment is overdue")]
    PaymentOverdue,

    #[msg("Cannot activate - status does not allow")]
    CannotActivate,

    #[msg("Cannot suspend - coverage not active")]
    CannotSuspend,

    #[msg("Cannot terminate - invalid status")]
    CannotTerminate,

    #[msg("Qualifying event not set")]
    NoQualifyingEvent,

    #[msg("Qualifying event expired")]
    QualifyingEventExpired,

    #[msg("Invalid enrollment window configuration")]
    InvalidWindowConfig,

    #[msg("Zone does not allow enrollment")]
    ZoneEnrollmentBlocked,

    #[msg("Special enrollment period required")]
    SpecialEnrollmentRequired,

    #[msg("Invalid benefit schedule")]
    InvalidBenefitSchedule,

    #[msg("Persistency discount not available yet")]
    PersistencyNotAvailable,

    #[msg("Maximum persistency discount reached")]
    MaxPersistencyReached,
}
