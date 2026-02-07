// programs/apollo_governance/src/instructions/emergency.rs

use crate::errors::GovernanceError;
use crate::events::{
    CommitteeUpdated, EmergencyActivated, EmergencyDeactivated, ProtocolPaused, ProtocolUnpaused,
};
use crate::state::{CommitteeType, DaoConfig, Multisig};
use anchor_lang::prelude::*;

/// Activate emergency mode - requires Risk Committee multisig
#[derive(Accounts)]
pub struct SetEmergencyFlag<'info> {
    #[account(
        mut,
        seeds = [DaoConfig::SEED_PREFIX],
        bump = dao_config.bump,
    )]
    pub dao_config: Account<'info, DaoConfig>,

    /// Risk Committee multisig
    #[account(
        seeds = [Multisig::SEED_PREFIX, b"risk_committee"],
        bump = risk_committee.bump,
        constraint = dao_config.risk_committee == risk_committee.key() @ GovernanceError::InvalidCommittee
    )]
    pub risk_committee: Account<'info, Multisig>,

    /// Must be a signer in the risk committee
    #[account(
        constraint = risk_committee.is_signer(&activator.key()) @ GovernanceError::Unauthorized
    )]
    pub activator: Signer<'info>,
}

pub fn activate_emergency(ctx: Context<SetEmergencyFlag>) -> Result<()> {
    let clock = Clock::get()?;
    let dao_config = &mut ctx.accounts.dao_config;

    // Don't require not already active - allow refresh
    dao_config.emergency_active = true;
    dao_config.emergency_activated_at = clock.unix_timestamp;

    let expires_at = clock.unix_timestamp + dao_config.max_emergency_duration;

    emit!(EmergencyActivated {
        activator: ctx.accounts.activator.key(),
        committee: CommitteeType::Risk,
        expires_at,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Deactivate emergency mode
#[derive(Accounts)]
pub struct DeactivateEmergency<'info> {
    #[account(
        mut,
        seeds = [DaoConfig::SEED_PREFIX],
        bump = dao_config.bump,
    )]
    pub dao_config: Account<'info, DaoConfig>,

    /// Can be deactivated by DAO authority or Risk Committee member
    pub deactivator: Signer<'info>,

    /// Optional: Risk Committee for member verification
    #[account(
        seeds = [Multisig::SEED_PREFIX, b"risk_committee"],
        bump = risk_committee.bump,
    )]
    pub risk_committee: Option<Account<'info, Multisig>>,
}

pub fn deactivate_emergency(ctx: Context<DeactivateEmergency>) -> Result<()> {
    let clock = Clock::get()?;
    let dao_config = &mut ctx.accounts.dao_config;

    // Verify authorization
    let is_authority = ctx.accounts.deactivator.key() == dao_config.authority;
    let is_risk_member = ctx
        .accounts
        .risk_committee
        .as_ref()
        .map(|rc| rc.is_signer(&ctx.accounts.deactivator.key()))
        .unwrap_or(false);

    require!(
        is_authority || is_risk_member,
        GovernanceError::Unauthorized
    );

    require!(
        dao_config.emergency_active,
        GovernanceError::EmergencyNotActive
    );

    let duration = clock.unix_timestamp - dao_config.emergency_activated_at;

    dao_config.emergency_active = false;
    dao_config.emergency_activated_at = 0;

    emit!(EmergencyDeactivated {
        deactivator: ctx.accounts.deactivator.key(),
        duration_seconds: duration,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Pause protocol - requires emergency active or DAO authority
#[derive(Accounts)]
pub struct PauseProtocol<'info> {
    #[account(
        mut,
        seeds = [DaoConfig::SEED_PREFIX],
        bump = dao_config.bump,
    )]
    pub dao_config: Account<'info, DaoConfig>,

    pub pauser: Signer<'info>,
}

pub fn pause_protocol(ctx: Context<PauseProtocol>) -> Result<()> {
    let clock = Clock::get()?;
    let dao_config = &mut ctx.accounts.dao_config;

    // Must be authority or emergency must be active
    let is_authority = ctx.accounts.pauser.key() == dao_config.authority;
    let emergency_valid =
        dao_config.emergency_active && !dao_config.is_emergency_expired(clock.unix_timestamp);

    require!(
        is_authority || emergency_valid,
        GovernanceError::Unauthorized
    );

    require!(!dao_config.protocol_paused, GovernanceError::ProtocolPaused);

    dao_config.protocol_paused = true;

    emit!(ProtocolPaused {
        pauser: ctx.accounts.pauser.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Unpause protocol
#[derive(Accounts)]
pub struct UnpauseProtocol<'info> {
    #[account(
        mut,
        seeds = [DaoConfig::SEED_PREFIX],
        bump = dao_config.bump,
    )]
    pub dao_config: Account<'info, DaoConfig>,

    #[account(
        constraint = unpauser.key() == dao_config.authority @ GovernanceError::Unauthorized
    )]
    pub unpauser: Signer<'info>,
}

pub fn unpause_protocol(ctx: Context<UnpauseProtocol>) -> Result<()> {
    let clock = Clock::get()?;
    let dao_config = &mut ctx.accounts.dao_config;

    require!(
        dao_config.protocol_paused,
        GovernanceError::ProtocolNotPaused
    );

    dao_config.protocol_paused = false;

    emit!(ProtocolUnpaused {
        unpauser: ctx.accounts.unpauser.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Update a committee address
#[derive(Accounts)]
pub struct UpdateCommittee<'info> {
    #[account(
        mut,
        seeds = [DaoConfig::SEED_PREFIX],
        bump = dao_config.bump,
    )]
    pub dao_config: Account<'info, DaoConfig>,

    #[account(
        constraint = authority.key() == dao_config.authority @ GovernanceError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn update_committee(
    ctx: Context<UpdateCommittee>,
    committee_type: CommitteeType,
    new_address: Pubkey,
) -> Result<()> {
    let clock = Clock::get()?;
    let dao_config = &mut ctx.accounts.dao_config;

    let old_address = match committee_type {
        CommitteeType::Risk => {
            let old = dao_config.risk_committee;
            dao_config.risk_committee = new_address;
            old
        }
        CommitteeType::Actuarial => {
            let old = dao_config.actuarial_committee;
            dao_config.actuarial_committee = new_address;
            old
        }
        CommitteeType::Claims => {
            let old = dao_config.claims_committee;
            dao_config.claims_committee = new_address;
            old
        }
        CommitteeType::Treasury => {
            let old = dao_config.treasury_committee;
            dao_config.treasury_committee = new_address;
            old
        }
        CommitteeType::Dao => {
            return Err(GovernanceError::InvalidCommittee.into());
        }
    };

    emit!(CommitteeUpdated {
        committee_type,
        old_address,
        new_address,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Update DAO authority (transfer ownership)
#[derive(Accounts)]
pub struct UpdateAuthority<'info> {
    #[account(
        mut,
        seeds = [DaoConfig::SEED_PREFIX],
        bump = dao_config.bump,
    )]
    pub dao_config: Account<'info, DaoConfig>,

    #[account(
        constraint = current_authority.key() == dao_config.authority @ GovernanceError::Unauthorized
    )]
    pub current_authority: Signer<'info>,

    /// CHECK: New authority doesn't need to sign
    pub new_authority: UncheckedAccount<'info>,
}

pub fn update_authority(ctx: Context<UpdateAuthority>) -> Result<()> {
    let dao_config = &mut ctx.accounts.dao_config;
    dao_config.authority = ctx.accounts.new_authority.key();
    Ok(())
}

/// Update max emergency duration
#[derive(Accounts)]
pub struct UpdateEmergencyDuration<'info> {
    #[account(
        mut,
        seeds = [DaoConfig::SEED_PREFIX],
        bump = dao_config.bump,
    )]
    pub dao_config: Account<'info, DaoConfig>,

    #[account(
        constraint = authority.key() == dao_config.authority @ GovernanceError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn update_emergency_duration(
    ctx: Context<UpdateEmergencyDuration>,
    new_duration: i64,
) -> Result<()> {
    require!(new_duration > 0, GovernanceError::InvalidExpiration);
    ctx.accounts.dao_config.max_emergency_duration = new_duration;
    Ok(())
}
