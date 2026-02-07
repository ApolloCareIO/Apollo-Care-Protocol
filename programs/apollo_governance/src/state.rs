// programs/apollo_governance/src/state.rs

use anchor_lang::prelude::*;

/// Global DAO configuration account
/// PDA seeds: ["dao_config"]
#[account]
#[derive(InitSpace)]
pub struct DaoConfig {
    /// Authority that can update DAO parameters (eventually the DAO itself)
    pub authority: Pubkey,

    /// Risk Committee multisig address
    pub risk_committee: Pubkey,

    /// Actuarial Committee multisig address
    pub actuarial_committee: Pubkey,

    /// Claims Committee multisig address
    pub claims_committee: Pubkey,

    /// Treasury Committee multisig address
    pub treasury_committee: Pubkey,

    /// Emergency flag - when true, enables emergency powers
    pub emergency_active: bool,

    /// Emergency activated timestamp (0 if not active)
    pub emergency_activated_at: i64,

    /// Maximum emergency duration in seconds (default: 48 hours)
    pub max_emergency_duration: i64,

    /// Total proposals created (counter)
    pub proposal_count: u64,

    /// Protocol paused flag
    pub protocol_paused: bool,

    /// Bump seed for PDA
    pub bump: u8,

    /// Reserved space for future upgrades
    #[max_len(64)]
    pub reserved: Vec<u8>,
}

impl DaoConfig {
    pub const SEED_PREFIX: &'static [u8] = b"dao_config";

    pub fn is_emergency_expired(&self, current_time: i64) -> bool {
        if !self.emergency_active {
            return true;
        }
        current_time > self.emergency_activated_at + self.max_emergency_duration
    }
}

/// Multisig account for committee-based authorization
/// PDA seeds: ["multisig", name]
#[account]
#[derive(InitSpace)]
pub struct Multisig {
    /// Name identifier for this multisig (e.g., "risk_committee")
    #[max_len(32)]
    pub name: String,

    /// Threshold required for execution (M of N)
    pub threshold: u8,

    /// Current number of signers
    pub signer_count: u8,

    /// Maximum signers allowed
    pub max_signers: u8,

    /// List of authorized signers
    #[max_len(11)]
    pub signers: Vec<Pubkey>,

    /// Number of transactions executed
    pub transaction_count: u64,

    /// Owner/creator of this multisig
    pub owner: Pubkey,

    /// Timestamp of creation
    pub created_at: i64,

    /// Is this multisig active
    pub is_active: bool,

    /// Bump seed for PDA
    pub bump: u8,
}

impl Multisig {
    pub const SEED_PREFIX: &'static [u8] = b"multisig";
    pub const MAX_SIGNERS: u8 = 11;

    /// Check if a given pubkey is a signer
    pub fn is_signer(&self, key: &Pubkey) -> bool {
        self.signers.contains(key)
    }

    /// Verify that required threshold of signers have signed
    pub fn verify_signatures(&self, signers_present: &[Pubkey]) -> bool {
        let valid_count = signers_present
            .iter()
            .filter(|s| self.is_signer(s))
            .count();
        valid_count >= self.threshold as usize
    }
}

/// Signer set for tracking signatures on a pending action
/// PDA seeds: ["signer_set", multisig, action_id]
#[account]
#[derive(InitSpace)]
pub struct SignerSet {
    /// Associated multisig
    pub multisig: Pubkey,

    /// Unique action identifier
    pub action_id: u64,

    /// Action type being authorized
    pub action_type: AdminAction,

    /// Target program/account for the action
    pub target: Pubkey,

    /// Signers who have approved this action
    #[max_len(11)]
    pub approvals: Vec<Pubkey>,

    /// Has this action been executed
    pub executed: bool,

    /// Creation timestamp
    pub created_at: i64,

    /// Expiration timestamp
    pub expires_at: i64,

    /// Action-specific data (serialized parameters)
    #[max_len(256)]
    pub action_data: Vec<u8>,

    /// Bump seed
    pub bump: u8,
}

impl SignerSet {
    pub const SEED_PREFIX: &'static [u8] = b"signer_set";
    pub const DEFAULT_EXPIRY: i64 = 7 * 24 * 60 * 60; // 7 days

    pub fn is_expired(&self, current_time: i64) -> bool {
        current_time > self.expires_at
    }

    pub fn has_approved(&self, signer: &Pubkey) -> bool {
        self.approvals.contains(signer)
    }

    pub fn approval_count(&self) -> usize {
        self.approvals.len()
    }
}

/// Stub proposal account for future full governance
/// PDA seeds: ["proposal", proposal_id]
#[account]
#[derive(InitSpace)]
pub struct Proposal {
    /// Unique proposal ID
    pub proposal_id: u64,

    /// Proposer address
    pub proposer: Pubkey,

    /// Title of the proposal
    #[max_len(128)]
    pub title: String,

    /// Description/IPFS hash
    #[max_len(256)]
    pub description_uri: String,

    /// Proposal type
    pub proposal_type: ProposalType,

    /// Current status
    pub status: ProposalStatus,

    /// Votes for
    pub votes_for: u64,

    /// Votes against
    pub votes_against: u64,

    /// Voting start timestamp
    pub voting_starts_at: i64,

    /// Voting end timestamp
    pub voting_ends_at: i64,

    /// Execution timestamp (0 if not executed)
    pub executed_at: i64,

    /// Quorum required (in basis points of total supply)
    pub quorum_bps: u16,

    /// Bump seed
    pub bump: u8,
}

impl Proposal {
    pub const SEED_PREFIX: &'static [u8] = b"proposal";
}

/// Admin action types for logging and authorization
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum AdminAction {
    // Governance actions
    UpdateDaoConfig,
    CreateMultisig,
    UpdateMultisig,
    AddSigner,
    RemoveSigner,
    SetThreshold,
    ActivateEmergency,
    DeactivateEmergency,
    PauseProtocol,
    UnpauseProtocol,

    // Risk engine actions
    UpdateRatingTable,
    SetShockFactor,
    SetEnrollmentCaps,
    UpdateCarState,

    // Reserve actions
    SetReserveTargets,
    UpdateIbnrParams,
    EmergencySpendRunoff,
    UpdateExpectedClaims,

    // Claims actions
    UpdateBenefitSchedule,
    ApproveClaim,
    DenyClaim,

    // Staking actions
    SetHaircutModel,
    QueueLiquidation,
    ExecuteLiquidation,
    SetMinLiquidityFlag,

    // Membership actions
    OpenEnrollmentWindow,
    CloseEnrollmentWindow,
    SetQualifyingEvent,
}

/// Proposal types for future governance
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum ProposalType {
    ParameterChange,
    EmergencyAction,
    TreasurySpend,
    CommitteeElection,
    ProtocolUpgrade,
    PolicyChange,
}

/// Proposal status
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum ProposalStatus {
    Draft,
    Active,
    Succeeded,
    Defeated,
    Executed,
    Cancelled,
    Expired,
}

/// Committee types for authorization
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum CommitteeType {
    Risk,
    Actuarial,
    Claims,
    Treasury,
    Dao,
}
