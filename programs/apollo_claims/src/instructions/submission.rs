// programs/apollo_claims/src/instructions/submission.rs

use anchor_lang::prelude::*;
use crate::state::{ClaimsConfig, ClaimAccount, ClaimStatus, ClaimCategory};
use crate::errors::ClaimsError;
use crate::events::{ClaimSubmitted, ClaimCancelled};

/// Submit a new claim
#[derive(Accounts)]
#[instruction(claim_id: u64)]
pub struct SubmitClaim<'info> {
    #[account(
        mut,
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump = claims_config.bump,
        constraint = claims_config.is_active @ ClaimsError::ClaimsPaused
    )]
    pub claims_config: Account<'info, ClaimsConfig>,

    #[account(
        init,
        payer = member,
        space = 8 + ClaimAccount::INIT_SPACE,
        seeds = [ClaimAccount::SEED_PREFIX, &claim_id.to_le_bytes()],
        bump
    )]
    pub claim: Account<'info, ClaimAccount>,

    #[account(mut)]
    pub member: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SubmitClaimParams {
    pub claim_id: u64,
    pub category: ClaimCategory,
    pub requested_amount: u64,
    pub service_date: i64,
    pub description_hash: String,
    pub provider: Option<Pubkey>,
}

pub fn submit_claim(ctx: Context<SubmitClaim>, params: SubmitClaimParams) -> Result<()> {
    let clock = Clock::get()?;
    let config = &mut ctx.accounts.claims_config;

    require!(params.requested_amount > 0, ClaimsError::InvalidClaimAmount);
    require!(params.service_date <= clock.unix_timestamp, ClaimsError::InvalidServiceDate);

    let is_shock = params.requested_amount >= config.shock_claim_threshold;

    let claim = &mut ctx.accounts.claim;
    claim.claim_id = params.claim_id;
    claim.member = ctx.accounts.member.key();
    claim.provider = params.provider;
    claim.category = params.category;
    claim.requested_amount = params.requested_amount;
    claim.approved_amount = 0;
    claim.paid_amount = 0;
    claim.status = ClaimStatus::Submitted;
    claim.submitted_at = clock.unix_timestamp;
    claim.status_changed_at = clock.unix_timestamp;
    claim.service_date = params.service_date;
    claim.description_hash = params.description_hash;
    claim.attestation_count = 0;
    claim.denial_reason = String::new();
    claim.is_shock_claim = is_shock;
    claim.bump = ctx.bumps.claim;

    config.total_claims_submitted += 1;

    emit!(ClaimSubmitted {
        claim_id: params.claim_id,
        member: ctx.accounts.member.key(),
        category: params.category,
        requested_amount: params.requested_amount,
        is_shock_claim: is_shock,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Cancel a claim (only by member, only in early status)
#[derive(Accounts)]
pub struct CancelClaim<'info> {
    #[account(
        mut,
        seeds = [ClaimAccount::SEED_PREFIX, &claim.claim_id.to_le_bytes()],
        bump = claim.bump,
        constraint = claim.member == member.key() @ ClaimsError::Unauthorized
    )]
    pub claim: Account<'info, ClaimAccount>,

    pub member: Signer<'info>,
}

pub fn cancel_claim(ctx: Context<CancelClaim>) -> Result<()> {
    let clock = Clock::get()?;
    let claim = &mut ctx.accounts.claim;

    // Can only cancel in early stages
    require!(
        matches!(
            claim.status,
            ClaimStatus::Submitted | ClaimStatus::UnderReview | ClaimStatus::PendingAttestation
        ),
        ClaimsError::CannotCancel
    );

    claim.status = ClaimStatus::Cancelled;
    claim.status_changed_at = clock.unix_timestamp;

    emit!(ClaimCancelled {
        claim_id: claim.claim_id,
        member: ctx.accounts.member.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Move claim to review status
#[derive(Accounts)]
pub struct MoveToReview<'info> {
    #[account(
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump = claims_config.bump,
    )]
    pub claims_config: Account<'info, ClaimsConfig>,

    #[account(
        mut,
        seeds = [ClaimAccount::SEED_PREFIX, &claim.claim_id.to_le_bytes()],
        bump = claim.bump,
        constraint = claim.status == ClaimStatus::Submitted @ ClaimsError::InvalidClaimStatus
    )]
    pub claim: Account<'info, ClaimAccount>,

    /// System or authorized reviewer
    pub reviewer: Signer<'info>,
}

pub fn move_to_review(ctx: Context<MoveToReview>) -> Result<()> {
    let clock = Clock::get()?;
    let claim = &mut ctx.accounts.claim;

    let old_status = claim.status;
    claim.status = ClaimStatus::UnderReview;
    claim.status_changed_at = clock.unix_timestamp;

    emit!(crate::events::ClaimStatusChanged {
        claim_id: claim.claim_id,
        old_status,
        new_status: ClaimStatus::UnderReview,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Move claim to pending attestation
#[derive(Accounts)]
pub struct MoveToPendingAttestation<'info> {
    #[account(
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump = claims_config.bump,
    )]
    pub claims_config: Account<'info, ClaimsConfig>,

    #[account(
        mut,
        seeds = [ClaimAccount::SEED_PREFIX, &claim.claim_id.to_le_bytes()],
        bump = claim.bump,
        constraint = claim.status == ClaimStatus::UnderReview @ ClaimsError::InvalidClaimStatus
    )]
    pub claim: Account<'info, ClaimAccount>,

    pub reviewer: Signer<'info>,
}

pub fn move_to_pending_attestation(ctx: Context<MoveToPendingAttestation>) -> Result<()> {
    let clock = Clock::get()?;
    let claim = &mut ctx.accounts.claim;
    let config = &ctx.accounts.claims_config;

    // Check if this should go to DAO vote instead
    if claim.is_shock_claim {
        claim.status = ClaimStatus::PendingDaoVote;
    } else if claim.requested_amount < config.auto_approve_threshold {
        // Could auto-approve small claims
        claim.status = ClaimStatus::PendingAttestation;
    } else {
        claim.status = ClaimStatus::PendingAttestation;
    }

    claim.status_changed_at = clock.unix_timestamp;

    Ok(())
}
