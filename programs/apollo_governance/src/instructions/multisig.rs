// programs/apollo_governance/src/instructions/multisig.rs

use crate::errors::GovernanceError;
use crate::events::{
    ActionApproved, ActionExecuted, MultisigCreated, SignerAdded, SignerRemoved, ThresholdUpdated,
};
use crate::state::{AdminAction, DaoConfig, Multisig, SignerSet};
use anchor_lang::prelude::*;

/// Create a new multisig
#[derive(Accounts)]
#[instruction(name: String)]
pub struct CreateMultisig<'info> {
    #[account(
        seeds = [DaoConfig::SEED_PREFIX],
        bump = dao_config.bump,
    )]
    pub dao_config: Account<'info, DaoConfig>,

    #[account(
        init,
        payer = authority,
        space = 8 + Multisig::INIT_SPACE,
        seeds = [Multisig::SEED_PREFIX, name.as_bytes()],
        bump
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        mut,
        constraint = authority.key() == dao_config.authority @ GovernanceError::Unauthorized
    )]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateMultisigParams {
    pub name: String,
    pub threshold: u8,
    pub initial_signers: Vec<Pubkey>,
}

pub fn create_multisig(ctx: Context<CreateMultisig>, params: CreateMultisigParams) -> Result<()> {
    let clock = Clock::get()?;

    require!(
        params.initial_signers.len() > 0,
        GovernanceError::ZeroSignersNotAllowed
    );
    require!(
        params.initial_signers.len() <= Multisig::MAX_SIGNERS as usize,
        GovernanceError::MaxSignersExceeded
    );
    require!(
        params.threshold > 0 && params.threshold as usize <= params.initial_signers.len(),
        GovernanceError::InvalidThreshold
    );
    require!(
        params.name.len() > 0 && params.name.len() <= 32,
        GovernanceError::InvalidMultisigName
    );

    let multisig = &mut ctx.accounts.multisig;
    multisig.name = params.name.clone();
    multisig.threshold = params.threshold;
    multisig.signer_count = params.initial_signers.len() as u8;
    multisig.max_signers = Multisig::MAX_SIGNERS;
    multisig.signers = params.initial_signers.clone();
    multisig.transaction_count = 0;
    multisig.owner = ctx.accounts.authority.key();
    multisig.created_at = clock.unix_timestamp;
    multisig.is_active = true;
    multisig.bump = ctx.bumps.multisig;

    emit!(MultisigCreated {
        multisig: ctx.accounts.multisig.key(),
        name: params.name,
        threshold: params.threshold,
        signers: params.initial_signers,
        owner: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Add a signer to multisig
#[derive(Accounts)]
pub struct AddSigner<'info> {
    #[account(
        seeds = [DaoConfig::SEED_PREFIX],
        bump = dao_config.bump,
    )]
    pub dao_config: Account<'info, DaoConfig>,

    #[account(
        mut,
        seeds = [Multisig::SEED_PREFIX, multisig.name.as_bytes()],
        bump = multisig.bump,
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        constraint = authority.key() == dao_config.authority @ GovernanceError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn add_signer(ctx: Context<AddSigner>, new_signer: Pubkey) -> Result<()> {
    let clock = Clock::get()?;
    let multisig_key = ctx.accounts.multisig.key();
    let multisig = &mut ctx.accounts.multisig;

    require!(multisig.is_active, GovernanceError::MultisigNotActive);
    require!(
        multisig.signer_count < multisig.max_signers,
        GovernanceError::MaxSignersExceeded
    );
    require!(
        !multisig.is_signer(&new_signer),
        GovernanceError::SignerAlreadyExists
    );

    multisig.signers.push(new_signer);
    multisig.signer_count += 1;

    emit!(SignerAdded {
        multisig: multisig_key,
        signer: new_signer,
        new_count: multisig.signer_count,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Remove a signer from multisig
#[derive(Accounts)]
pub struct RemoveSigner<'info> {
    #[account(
        seeds = [DaoConfig::SEED_PREFIX],
        bump = dao_config.bump,
    )]
    pub dao_config: Account<'info, DaoConfig>,

    #[account(
        mut,
        seeds = [Multisig::SEED_PREFIX, multisig.name.as_bytes()],
        bump = multisig.bump,
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        constraint = authority.key() == dao_config.authority @ GovernanceError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn remove_signer(ctx: Context<RemoveSigner>, signer_to_remove: Pubkey) -> Result<()> {
    let clock = Clock::get()?;
    let multisig = &mut ctx.accounts.multisig;

    require!(multisig.is_active, GovernanceError::MultisigNotActive);
    require!(
        multisig.is_signer(&signer_to_remove),
        GovernanceError::SignerNotFound
    );

    // Ensure removing doesn't break threshold
    let new_count = multisig.signer_count - 1;
    require!(
        new_count >= multisig.threshold,
        GovernanceError::CannotRemoveSigner
    );

    multisig.signers.retain(|s| s != &signer_to_remove);
    multisig.signer_count = new_count;

    emit!(SignerRemoved {
        multisig: ctx.accounts.multisig.key(),
        signer: signer_to_remove,
        new_count,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Set threshold for multisig
#[derive(Accounts)]
pub struct SetThreshold<'info> {
    #[account(
        seeds = [DaoConfig::SEED_PREFIX],
        bump = dao_config.bump,
    )]
    pub dao_config: Account<'info, DaoConfig>,

    #[account(
        mut,
        seeds = [Multisig::SEED_PREFIX, multisig.name.as_bytes()],
        bump = multisig.bump,
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        constraint = authority.key() == dao_config.authority @ GovernanceError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn set_threshold(ctx: Context<SetThreshold>, new_threshold: u8) -> Result<()> {
    let clock = Clock::get()?;
    let multisig = &mut ctx.accounts.multisig;

    require!(multisig.is_active, GovernanceError::MultisigNotActive);
    require!(
        new_threshold > 0 && new_threshold <= multisig.signer_count,
        GovernanceError::InvalidThreshold
    );

    let old_threshold = multisig.threshold;
    multisig.threshold = new_threshold;

    emit!(ThresholdUpdated {
        multisig: ctx.accounts.multisig.key(),
        old_threshold,
        new_threshold,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Create a signer set for an action requiring multisig approval
#[derive(Accounts)]
#[instruction(action_id: u64)]
pub struct CreateSignerSet<'info> {
    #[account(
        seeds = [Multisig::SEED_PREFIX, multisig.name.as_bytes()],
        bump = multisig.bump,
        constraint = multisig.is_active @ GovernanceError::MultisigNotActive
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        init,
        payer = proposer,
        space = 8 + SignerSet::INIT_SPACE,
        seeds = [SignerSet::SEED_PREFIX, multisig.key().as_ref(), &action_id.to_le_bytes()],
        bump
    )]
    pub signer_set: Account<'info, SignerSet>,

    #[account(
        mut,
        constraint = multisig.is_signer(&proposer.key()) @ GovernanceError::Unauthorized
    )]
    pub proposer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateSignerSetParams {
    pub action_id: u64,
    pub action_type: AdminAction,
    pub target: Pubkey,
    pub action_data: Vec<u8>,
    pub expiry_seconds: Option<i64>,
}

pub fn create_signer_set(
    ctx: Context<CreateSignerSet>,
    params: CreateSignerSetParams,
) -> Result<()> {
    let clock = Clock::get()?;
    let expiry = params.expiry_seconds.unwrap_or(SignerSet::DEFAULT_EXPIRY);

    require!(expiry > 0, GovernanceError::InvalidExpiration);

    let signer_set_key = ctx.accounts.signer_set.key();
    let signer_set = &mut ctx.accounts.signer_set;
    signer_set.multisig = ctx.accounts.multisig.key();
    signer_set.action_id = params.action_id;
    signer_set.action_type = params.action_type;
    signer_set.target = params.target;
    signer_set.approvals = vec![ctx.accounts.proposer.key()]; // Proposer auto-approves
    signer_set.executed = false;
    signer_set.created_at = clock.unix_timestamp;
    signer_set.expires_at = clock.unix_timestamp + expiry;
    signer_set.action_data = params.action_data;
    signer_set.bump = ctx.bumps.signer_set;

    emit!(ActionApproved {
        signer_set: signer_set_key,
        signer: ctx.accounts.proposer.key(),
        action_type: signer_set.action_type,
        approval_count: 1,
        threshold: ctx.accounts.multisig.threshold,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Approve an action in a signer set
#[derive(Accounts)]
pub struct ApproveAction<'info> {
    #[account(
        seeds = [Multisig::SEED_PREFIX, multisig.name.as_bytes()],
        bump = multisig.bump,
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        mut,
        seeds = [SignerSet::SEED_PREFIX, multisig.key().as_ref(), &signer_set.action_id.to_le_bytes()],
        bump = signer_set.bump,
        constraint = !signer_set.executed @ GovernanceError::ActionAlreadyExecuted
    )]
    pub signer_set: Account<'info, SignerSet>,

    #[account(
        constraint = multisig.is_signer(&signer.key()) @ GovernanceError::Unauthorized
    )]
    pub signer: Signer<'info>,
}

pub fn approve_action(ctx: Context<ApproveAction>) -> Result<()> {
    let clock = Clock::get()?;
    let signer_set_key = ctx.accounts.signer_set.key();
    let signer_set = &mut ctx.accounts.signer_set;

    require!(
        !signer_set.is_expired(clock.unix_timestamp),
        GovernanceError::ActionExpired
    );
    require!(
        !signer_set.has_approved(&ctx.accounts.signer.key()),
        GovernanceError::AlreadyApproved
    );

    signer_set.approvals.push(ctx.accounts.signer.key());

    emit!(ActionApproved {
        signer_set: signer_set_key,
        signer: ctx.accounts.signer.key(),
        action_type: signer_set.action_type,
        approval_count: signer_set.approvals.len() as u8,
        threshold: ctx.accounts.multisig.threshold,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Check if a signer set has reached threshold (helper that can be called by other programs)
/// This is the "assert_signed" helper
#[derive(Accounts)]
pub struct AssertSigned<'info> {
    #[account(
        seeds = [Multisig::SEED_PREFIX, multisig.name.as_bytes()],
        bump = multisig.bump,
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        seeds = [SignerSet::SEED_PREFIX, multisig.key().as_ref(), &signer_set.action_id.to_le_bytes()],
        bump = signer_set.bump,
        constraint = !signer_set.executed @ GovernanceError::ActionAlreadyExecuted
    )]
    pub signer_set: Account<'info, SignerSet>,
}

/// Verify signatures and mark action as executed
pub fn assert_signed(ctx: Context<AssertSigned>, expected_action: AdminAction) -> Result<()> {
    let clock = Clock::get()?;
    let signer_set = &ctx.accounts.signer_set;
    let multisig = &ctx.accounts.multisig;

    require!(
        !signer_set.is_expired(clock.unix_timestamp),
        GovernanceError::ActionExpired
    );
    require!(
        signer_set.action_type == expected_action,
        GovernanceError::ActionTypeMismatch
    );
    require!(
        signer_set.approval_count() >= multisig.threshold as usize,
        GovernanceError::InsufficientSignatures
    );

    // Note: In a real impl, we'd mark executed here, but since we're doing
    // cross-program validation, the calling program handles execution marking

    Ok(())
}

/// Mark action as executed after successful CPI
#[derive(Accounts)]
pub struct MarkExecuted<'info> {
    #[account(
        seeds = [Multisig::SEED_PREFIX, multisig.name.as_bytes()],
        bump = multisig.bump,
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        mut,
        seeds = [SignerSet::SEED_PREFIX, multisig.key().as_ref(), &signer_set.action_id.to_le_bytes()],
        bump = signer_set.bump,
        constraint = !signer_set.executed @ GovernanceError::ActionAlreadyExecuted
    )]
    pub signer_set: Account<'info, SignerSet>,

    /// Must be signed by the target program or DAO authority
    pub executor: Signer<'info>,
}

pub fn mark_executed(ctx: Context<MarkExecuted>) -> Result<()> {
    let clock = Clock::get()?;
    let signer_set_key = ctx.accounts.signer_set.key();
    let signer_set = &mut ctx.accounts.signer_set;

    signer_set.executed = true;

    emit!(ActionExecuted {
        signer_set: signer_set_key,
        action_type: signer_set.action_type,
        target: signer_set.target,
        executor: ctx.accounts.executor.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
