// programs/apollo_governance/src/events.rs

use anchor_lang::prelude::*;
use crate::state::{AdminAction, CommitteeType};

/// Emitted when DAO is initialized
#[event]
pub struct DaoInitialized {
    pub authority: Pubkey,
    pub timestamp: i64,
}

/// Emitted when a multisig is created
#[event]
pub struct MultisigCreated {
    pub multisig: Pubkey,
    pub name: String,
    pub threshold: u8,
    pub signers: Vec<Pubkey>,
    pub owner: Pubkey,
    pub timestamp: i64,
}

/// Emitted when a signer is added to multisig
#[event]
pub struct SignerAdded {
    pub multisig: Pubkey,
    pub signer: Pubkey,
    pub new_count: u8,
    pub timestamp: i64,
}

/// Emitted when a signer is removed from multisig
#[event]
pub struct SignerRemoved {
    pub multisig: Pubkey,
    pub signer: Pubkey,
    pub new_count: u8,
    pub timestamp: i64,
}

/// Emitted when threshold is updated
#[event]
pub struct ThresholdUpdated {
    pub multisig: Pubkey,
    pub old_threshold: u8,
    pub new_threshold: u8,
    pub timestamp: i64,
}

/// Emitted when an action is approved by a signer
#[event]
pub struct ActionApproved {
    pub signer_set: Pubkey,
    pub signer: Pubkey,
    pub action_type: AdminAction,
    pub approval_count: u8,
    pub threshold: u8,
    pub timestamp: i64,
}

/// Emitted when an action is executed
#[event]
pub struct ActionExecuted {
    pub signer_set: Pubkey,
    pub action_type: AdminAction,
    pub target: Pubkey,
    pub executor: Pubkey,
    pub timestamp: i64,
}

/// Emitted when emergency mode is activated
#[event]
pub struct EmergencyActivated {
    pub activator: Pubkey,
    pub committee: CommitteeType,
    pub expires_at: i64,
    pub timestamp: i64,
}

/// Emitted when emergency mode is deactivated
#[event]
pub struct EmergencyDeactivated {
    pub deactivator: Pubkey,
    pub duration_seconds: i64,
    pub timestamp: i64,
}

/// Emitted when protocol is paused
#[event]
pub struct ProtocolPaused {
    pub pauser: Pubkey,
    pub timestamp: i64,
}

/// Emitted when protocol is unpaused
#[event]
pub struct ProtocolUnpaused {
    pub unpauser: Pubkey,
    pub timestamp: i64,
}

/// Emitted when committee is updated
#[event]
pub struct CommitteeUpdated {
    pub committee_type: CommitteeType,
    pub old_address: Pubkey,
    pub new_address: Pubkey,
    pub timestamp: i64,
}

/// Emitted when a proposal is created (stub)
#[event]
pub struct ProposalCreated {
    pub proposal_id: u64,
    pub proposer: Pubkey,
    pub title: String,
    pub voting_starts_at: i64,
    pub voting_ends_at: i64,
    pub timestamp: i64,
}

/// Emitted when DAO config is updated
#[event]
pub struct DaoConfigUpdated {
    pub field: String,
    pub updater: Pubkey,
    pub timestamp: i64,
}
