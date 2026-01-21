// programs/apollo_risk_engine/src/errors.rs

use anchor_lang::prelude::*;

#[error_code]
pub enum RiskEngineError {
    #[msg("Risk engine not initialized")]
    NotInitialized,

    #[msg("Risk engine already initialized")]
    AlreadyInitialized,

    #[msg("Unauthorized: caller lacks permission")]
    Unauthorized,

    #[msg("Invalid age band configuration")]
    InvalidAgeBand,

    #[msg("Age bands must not overlap")]
    OverlappingAgeBands,

    #[msg("Age band ratio exceeds CMS 3:1 limit")]
    AgeBandRatioExceeded,

    #[msg("Invalid region factor")]
    InvalidRegionFactor,

    #[msg("Invalid ShockFactor: exceeds maximum for current zone")]
    ShockFactorExceedsZoneLimit,

    #[msg("ShockFactor change requires Risk Committee approval")]
    ShockFactorRequiresCommittee,

    #[msg("ShockFactor change requires DAO emergency approval")]
    ShockFactorRequiresEmergency,

    #[msg("Invalid basis points value")]
    InvalidBasisPoints,

    #[msg("CAR below minimum threshold")]
    CarBelowMinimum,

    #[msg("Enrollment frozen due to Red zone")]
    EnrollmentFrozen,

    #[msg("Enrollment cap exceeded for current zone")]
    EnrollmentCapExceeded,

    #[msg("Invalid zone configuration")]
    InvalidZoneConfig,

    #[msg("Zone transition not allowed")]
    InvalidZoneTransition,

    #[msg("Math overflow in calculation")]
    MathOverflow,

    #[msg("Invalid member age")]
    InvalidAge,

    #[msg("Invalid tobacco factor")]
    InvalidTobaccoFactor,

    #[msg("Expected claims cannot be zero")]
    ZeroExpectedClaims,

    #[msg("Rating table is empty")]
    EmptyRatingTable,

    #[msg("Too many age bands")]
    TooManyAgeBands,

    #[msg("Too many regions")]
    TooManyRegions,

    #[msg("Contribution below minimum")]
    ContributionBelowMinimum,

    #[msg("Risk engine is not active")]
    RiskEngineNotActive,
}
