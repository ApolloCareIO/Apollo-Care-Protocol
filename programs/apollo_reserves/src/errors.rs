// programs/apollo_reserves/src/errors.rs

use anchor_lang::prelude::*;

#[error_code]
pub enum ReservesError {
    #[msg("Reserves not initialized")]
    NotInitialized,

    #[msg("Reserves already initialized")]
    AlreadyInitialized,

    #[msg("Unauthorized: caller lacks permission")]
    Unauthorized,

    #[msg("Insufficient funds in Tier 0")]
    InsufficientTier0,

    #[msg("Insufficient funds in Tier 1")]
    InsufficientTier1,

    #[msg("Insufficient funds in Tier 2")]
    InsufficientTier2,

    #[msg("Insufficient total reserves for payout")]
    InsufficientReserves,

    #[msg("Run-off reserve is not accessible in normal operations")]
    RunoffNotAccessible,

    #[msg("Run-off mode is not active")]
    RunoffNotActive,

    #[msg("Run-off mode is already active")]
    RunoffAlreadyActive,

    #[msg("Emergency flag required for this action")]
    EmergencyRequired,

    #[msg("Invalid vault configuration")]
    InvalidVaultConfig,

    #[msg("Invalid reserve target days")]
    InvalidTargetDays,

    #[msg("Invalid basis points value (must be <= 10000)")]
    InvalidBasisPoints,

    #[msg("Invalid IBNR parameters")]
    InvalidIbnrParams,

    #[msg("Math overflow in calculation")]
    MathOverflow,

    #[msg("Reserve coverage below minimum")]
    CoverageBelowMinimum,

    #[msg("Waterfall exhausted - all tiers depleted")]
    WaterfallExhausted,

    #[msg("Invalid payout amount")]
    InvalidPayoutAmount,

    #[msg("Zero amount not allowed")]
    ZeroAmount,

    #[msg("Contribution routing sum mismatch")]
    RoutingMismatch,

    #[msg("Protocol is paused")]
    ProtocolPaused,

    #[msg("Invalid mint - expected USDC")]
    InvalidMint,

    #[msg("Vault authority mismatch")]
    VaultAuthorityMismatch,

    #[msg("Expected claims cannot be zero")]
    ZeroExpectedClaims,

    #[msg("Development factor must be >= 10000 (1.0)")]
    InvalidDevFactor,
    
    // Phase Management Errors
    #[msg("Invalid phase transition - must be sequential")]
    InvalidPhaseTransition,
    
    #[msg("Invalid phase for this operation")]
    InvalidPhase,
    
    #[msg("Phase transition already pending")]
    TransitionAlreadyPending,
    
    #[msg("No phase transition pending")]
    NoTransitionPending,
    
    #[msg("Phase requirements not met")]
    PhaseRequirementsNotMet,
    
    #[msg("Smart contract or financial audit not complete")]
    AuditNotComplete,
    
    #[msg("Regulatory approval required for this transition")]
    RegulatoryApprovalRequired,
    
    #[msg("Insufficient capital for regulatory requirements")]
    InsufficientCapital,
    
    #[msg("Actuarial certification required")]
    CertificationRequired,
    
    #[msg("Required committees not established")]
    CommitteesRequired,
    
    // Reinsurance Errors
    #[msg("Reinsurance is inactive")]
    ReinsuranceInactive,
    
    #[msg("Reinsurance policy expired")]
    ReinsurancePolicyExpired,
    
    #[msg("Invalid reinsurance configuration")]
    InvalidReinsuranceConfig,
    
    // Cohort Errors
    #[msg("Cohort not found")]
    CohortNotFound,
    
    #[msg("Cohort flagged for adverse selection")]
    CohortFlagged,
}

// Re-export for backwards compatibility
pub use ReservesError as ReserveError;
