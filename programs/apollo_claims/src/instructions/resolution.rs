// programs/apollo_claims/src/instructions/resolution.rs

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use crate::state::{ClaimsConfig, ClaimAccount, ClaimStatus};
use crate::errors::ClaimsError;
use crate::events::{ClaimApproved, ClaimDenied, ClaimPaid, ClaimClosed, ClaimAppealed};

/// Approve a claim
#[derive(Accounts)]
pub struct ApproveClaim<'info> {
    #[account(
        mut,
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump = claims_config.bump,
    )]
    pub claims_config: Account<'info, ClaimsConfig>,

    #[account(
        mut,
        seeds = [ClaimAccount::SEED_PREFIX, &claim.claim_id.to_le_bytes()],
        bump = claim.bump,
        constraint = matches!(
            claim.status,
            ClaimStatus::PendingAttestation | ClaimStatus::PendingDaoVote | ClaimStatus::Appealed
        ) @ ClaimsError::InvalidClaimStatus
    )]
    pub claim: Account<'info, ClaimAccount>,

    /// Must be authorized (committee for regular, DAO for shock claims)
    #[account(
        constraint = approver.key() == claims_config.authority ||
                     approver.key() == claims_config.claims_committee @ ClaimsError::Unauthorized
    )]
    pub approver: Signer<'info>,
}

pub fn approve_claim(ctx: Context<ApproveClaim>, approved_amount: u64) -> Result<()> {
    let clock = Clock::get()?;
    let config = &mut ctx.accounts.claims_config;
    let claim = &mut ctx.accounts.claim;

    // Verify sufficient attestations for non-shock claims
    if !claim.is_shock_claim && claim.status == ClaimStatus::PendingAttestation {
        require!(
            claim.attestation_count >= config.required_attestations,
            ClaimsError::InsufficientAttestations
        );
    }

    // Approved amount can be less than requested but not zero for approval
    require!(approved_amount > 0, ClaimsError::InvalidClaimAmount);
    require!(approved_amount <= claim.requested_amount, ClaimsError::InvalidClaimAmount);

    claim.approved_amount = approved_amount;
    claim.status = ClaimStatus::Approved;
    claim.status_changed_at = clock.unix_timestamp;

    config.total_claims_approved += 1;

    emit!(ClaimApproved {
        claim_id: claim.claim_id,
        member: claim.member,
        requested_amount: claim.requested_amount,
        approved_amount,
        approver: ctx.accounts.approver.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Deny a claim
#[derive(Accounts)]
pub struct DenyClaim<'info> {
    #[account(
        mut,
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump = claims_config.bump,
    )]
    pub claims_config: Account<'info, ClaimsConfig>,

    #[account(
        mut,
        seeds = [ClaimAccount::SEED_PREFIX, &claim.claim_id.to_le_bytes()],
        bump = claim.bump,
        constraint = matches!(
            claim.status,
            ClaimStatus::PendingAttestation | ClaimStatus::PendingDaoVote | ClaimStatus::UnderReview
        ) @ ClaimsError::InvalidClaimStatus
    )]
    pub claim: Account<'info, ClaimAccount>,

    #[account(
        constraint = denier.key() == claims_config.authority ||
                     denier.key() == claims_config.claims_committee @ ClaimsError::Unauthorized
    )]
    pub denier: Signer<'info>,
}

pub fn deny_claim(ctx: Context<DenyClaim>, reason: String) -> Result<()> {
    let clock = Clock::get()?;
    let config = &mut ctx.accounts.claims_config;
    let claim = &mut ctx.accounts.claim;

    claim.status = ClaimStatus::Denied;
    claim.denial_reason = reason.clone();
    claim.status_changed_at = clock.unix_timestamp;

    config.total_claims_denied += 1;

    emit!(ClaimDenied {
        claim_id: claim.claim_id,
        member: claim.member,
        requested_amount: claim.requested_amount,
        reason,
        denier: ctx.accounts.denier.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Pay an approved claim (calls reserves for payout)
#[derive(Accounts)]
pub struct PayClaim<'info> {
    #[account(
        mut,
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump = claims_config.bump,
    )]
    pub claims_config: Account<'info, ClaimsConfig>,

    #[account(
        mut,
        seeds = [ClaimAccount::SEED_PREFIX, &claim.claim_id.to_le_bytes()],
        bump = claim.bump,
        constraint = claim.status == ClaimStatus::Approved @ ClaimsError::InvalidClaimStatus
    )]
    pub claim: Account<'info, ClaimAccount>,

    /// Recipient token account (member or provider)
    #[account(mut)]
    pub recipient: Account<'info, TokenAccount>,

    // NOTE: In production, this would include CPI to reserves program
    // For scaffold, we just update state and emit events

    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn pay_claim(ctx: Context<PayClaim>) -> Result<()> {
    let clock = Clock::get()?;
    let config = &mut ctx.accounts.claims_config;
    let claim = &mut ctx.accounts.claim;

    require!(claim.approved_amount > 0, ClaimsError::InvalidClaimAmount);
    require!(claim.paid_amount < claim.approved_amount, ClaimsError::AlreadyPaid);

    let payment_amount = claim.approved_amount - claim.paid_amount;

    // TODO: CPI to apollo_reserves::payout_claim_from_waterfall
    // For scaffold, we mark as paid without actual transfer
    // Production would execute:
    // apollo_reserves::cpi::payout_claim_from_waterfall(
    //     cpi_ctx,
    //     PayoutParams { claim_id: claim.claim_id, amount: payment_amount }
    // )?;

    claim.paid_amount = claim.approved_amount;
    claim.status = ClaimStatus::Paid;
    claim.status_changed_at = clock.unix_timestamp;

    config.total_paid_out = config.total_paid_out.saturating_add(payment_amount);

    emit!(ClaimPaid {
        claim_id: claim.claim_id,
        member: claim.member,
        amount: payment_amount,
        recipient: ctx.accounts.recipient.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Close a claim (finalize)
#[derive(Accounts)]
pub struct CloseClaim<'info> {
    #[account(
        mut,
        seeds = [ClaimAccount::SEED_PREFIX, &claim.claim_id.to_le_bytes()],
        bump = claim.bump,
        constraint = matches!(
            claim.status,
            ClaimStatus::Paid | ClaimStatus::Denied | ClaimStatus::Cancelled
        ) @ ClaimsError::InvalidClaimStatus
    )]
    pub claim: Account<'info, ClaimAccount>,

    pub closer: Signer<'info>,
}

pub fn close_claim(ctx: Context<CloseClaim>) -> Result<()> {
    let clock = Clock::get()?;
    let claim = &mut ctx.accounts.claim;

    let final_status = claim.status;
    claim.status = ClaimStatus::Closed;
    claim.status_changed_at = clock.unix_timestamp;

    emit!(ClaimClosed {
        claim_id: claim.claim_id,
        final_status,
        total_paid: claim.paid_amount,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Appeal a denied claim
#[derive(Accounts)]
pub struct AppealClaim<'info> {
    #[account(
        mut,
        seeds = [ClaimAccount::SEED_PREFIX, &claim.claim_id.to_le_bytes()],
        bump = claim.bump,
        constraint = claim.status == ClaimStatus::Denied @ ClaimsError::InvalidClaimStatus,
        constraint = claim.member == member.key() @ ClaimsError::Unauthorized
    )]
    pub claim: Account<'info, ClaimAccount>,

    pub member: Signer<'info>,
}

pub fn appeal_claim(ctx: Context<AppealClaim>) -> Result<()> {
    let clock = Clock::get()?;
    let claim = &mut ctx.accounts.claim;

    let previous_status = claim.status;
    claim.status = ClaimStatus::Appealed;
    claim.status_changed_at = clock.unix_timestamp;
    claim.attestation_count = 0; // Reset for re-review

    emit!(ClaimAppealed {
        claim_id: claim.claim_id,
        member: ctx.accounts.member.key(),
        previous_status,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
