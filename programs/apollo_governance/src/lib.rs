// programs/apollo_governance/src/lib.rs
//
// Apollo Governance Program
// =========================
// Provides DAO structure, multisig authorization, and emergency controls
// for the Apollo Care protocol. All privileged actions across other programs
// must be authorized through this governance layer.

use anchor_lang::prelude::*;

pub mod state;
pub mod errors;
pub mod events;
pub mod instructions;

use instructions::*;
use state::{AdminAction, CommitteeType};

declare_id!("HynmZCjBZ5eHXL48Z7db6CwiCjh6KMXnCHXrsP11Vzdd");

#[program]
pub mod apollo_governance {
    use super::*;

    // ==================== DAO INITIALIZATION ====================

    /// Initialize the DAO configuration
    /// This is called once at protocol launch to set up the governance structure
    pub fn initialize_dao(ctx: Context<InitializeDao>, params: InitializeDaoParams) -> Result<()> {
        instructions::initialize::handler(ctx, params)
    }

    // ==================== MULTISIG MANAGEMENT ====================

    /// Create a new multisig for a committee or special purpose
    pub fn create_multisig(ctx: Context<CreateMultisig>, params: CreateMultisigParams) -> Result<()> {
        instructions::multisig::create_multisig(ctx, params)
    }

    /// Add a signer to an existing multisig
    pub fn add_signer(ctx: Context<AddSigner>, new_signer: Pubkey) -> Result<()> {
        instructions::multisig::add_signer(ctx, new_signer)
    }

    /// Remove a signer from a multisig
    pub fn remove_signer(ctx: Context<RemoveSigner>, signer_to_remove: Pubkey) -> Result<()> {
        instructions::multisig::remove_signer(ctx, signer_to_remove)
    }

    /// Update the threshold requirement for a multisig
    pub fn set_threshold(ctx: Context<SetThreshold>, new_threshold: u8) -> Result<()> {
        instructions::multisig::set_threshold(ctx, new_threshold)
    }

    // ==================== ACTION AUTHORIZATION ====================

    /// Create a signer set for a pending multisig action
    pub fn create_signer_set(ctx: Context<CreateSignerSet>, params: CreateSignerSetParams) -> Result<()> {
        instructions::multisig::create_signer_set(ctx, params)
    }

    /// Approve a pending action (add signature to signer set)
    pub fn approve_action(ctx: Context<ApproveAction>) -> Result<()> {
        instructions::multisig::approve_action(ctx)
    }

    /// Verify that a signer set has reached threshold
    /// Called by other programs to validate authorization
    pub fn assert_signed(ctx: Context<AssertSigned>, expected_action: AdminAction) -> Result<()> {
        instructions::multisig::assert_signed(ctx, expected_action)
    }

    /// Mark an action as executed after successful CPI
    pub fn mark_executed(ctx: Context<MarkExecuted>) -> Result<()> {
        instructions::multisig::mark_executed(ctx)
    }

    // ==================== EMERGENCY CONTROLS ====================

    /// Activate emergency mode - enables emergency powers for Risk Committee
    pub fn set_emergency_flag(ctx: Context<SetEmergencyFlag>) -> Result<()> {
        instructions::emergency::activate_emergency(ctx)
    }

    /// Deactivate emergency mode
    pub fn deactivate_emergency(ctx: Context<DeactivateEmergency>) -> Result<()> {
        instructions::emergency::deactivate_emergency(ctx)
    }

    /// Pause the entire protocol
    pub fn pause_protocol(ctx: Context<PauseProtocol>) -> Result<()> {
        instructions::emergency::pause_protocol(ctx)
    }

    /// Unpause the protocol
    pub fn unpause_protocol(ctx: Context<UnpauseProtocol>) -> Result<()> {
        instructions::emergency::unpause_protocol(ctx)
    }

    // ==================== DAO CONFIGURATION ====================

    /// Update a committee's multisig address
    pub fn update_committee(
        ctx: Context<UpdateCommittee>,
        committee_type: CommitteeType,
        new_address: Pubkey
    ) -> Result<()> {
        instructions::emergency::update_committee(ctx, committee_type, new_address)
    }

    /// Transfer DAO authority to a new address
    pub fn update_authority(ctx: Context<UpdateAuthority>) -> Result<()> {
        instructions::emergency::update_authority(ctx)
    }

    /// Update the maximum emergency duration
    pub fn update_emergency_duration(ctx: Context<UpdateEmergencyDuration>, new_duration: i64) -> Result<()> {
        instructions::emergency::update_emergency_duration(ctx, new_duration)
    }
}

/// Helper module for CPI authorization checks
/// Other programs use these functions to verify multisig approval
pub mod authorization {
    use super::*;
    

    /// Seeds for DAO config PDA
    pub fn dao_config_seeds() -> &'static [&'static [u8]] {
        &[state::DaoConfig::SEED_PREFIX]
    }

    /// Seeds for multisig PDA
    pub fn multisig_seeds(name: &[u8]) -> [&[u8]; 2] {
        [state::Multisig::SEED_PREFIX, name]
    }

    /// Seeds for signer set PDA
    pub fn signer_set_seeds<'a>(multisig: &'a [u8], action_id: &'a [u8]) -> [&'a [u8]; 3] {
        [state::SignerSet::SEED_PREFIX, multisig, action_id]
    }

    /// Check if protocol is paused (for use in constraint checks)
    pub fn is_protocol_active(dao_config: &state::DaoConfig) -> bool {
        !dao_config.protocol_paused
    }

    /// Check if emergency mode is active and not expired
    pub fn is_emergency_active(dao_config: &state::DaoConfig, current_time: i64) -> bool {
        dao_config.emergency_active && !dao_config.is_emergency_expired(current_time)
    }
}
