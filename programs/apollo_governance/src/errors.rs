// programs/apollo_governance/src/errors.rs

use anchor_lang::prelude::*;

#[error_code]
pub enum GovernanceError {
    #[msg("Unauthorized: caller is not the DAO authority")]
    Unauthorized,

    #[msg("Invalid threshold: must be > 0 and <= signer count")]
    InvalidThreshold,

    #[msg("Max signers exceeded")]
    MaxSignersExceeded,

    #[msg("Signer already exists in multisig")]
    SignerAlreadyExists,

    #[msg("Signer not found in multisig")]
    SignerNotFound,

    #[msg("Cannot remove signer: would break threshold")]
    CannotRemoveSigner,

    #[msg("Multisig is not active")]
    MultisigNotActive,

    #[msg("Insufficient signatures for action")]
    InsufficientSignatures,

    #[msg("Action has already been executed")]
    ActionAlreadyExecuted,

    #[msg("Action has expired")]
    ActionExpired,

    #[msg("Signer has already approved this action")]
    AlreadyApproved,

    #[msg("Emergency mode is not active")]
    EmergencyNotActive,

    #[msg("Emergency mode has expired")]
    EmergencyExpired,

    #[msg("Emergency mode is still active")]
    EmergencyStillActive,

    #[msg("Protocol is paused")]
    ProtocolPaused,

    #[msg("Protocol is not paused")]
    ProtocolNotPaused,

    #[msg("Invalid committee type")]
    InvalidCommittee,

    #[msg("Invalid multisig name")]
    InvalidMultisigName,

    #[msg("Proposal not found")]
    ProposalNotFound,

    #[msg("Proposal voting has not started")]
    VotingNotStarted,

    #[msg("Proposal voting has ended")]
    VotingEnded,

    #[msg("Proposal is not in active status")]
    ProposalNotActive,

    #[msg("Quorum not reached")]
    QuorumNotReached,

    #[msg("Invalid action data")]
    InvalidActionData,

    #[msg("Action type mismatch")]
    ActionTypeMismatch,

    #[msg("Zero signers not allowed")]
    ZeroSignersNotAllowed,

    #[msg("Invalid expiration time")]
    InvalidExpiration,
}
