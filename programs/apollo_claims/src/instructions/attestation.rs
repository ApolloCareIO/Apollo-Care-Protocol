// programs/apollo_claims/src/instructions/attestation.rs

use anchor_lang::prelude::*;
use crate::state::{
    ClaimsConfig, ClaimAccount, ClaimStatus,
    AttestorRegistry, Attestation, AttestationRecommendation
};
use crate::errors::ClaimsError;
use crate::events::ClaimAttested;

/// Attest a claim (committee member review)
#[derive(Accounts)]
pub struct AttestClaim<'info> {
    #[account(
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump = claims_config.bump,
    )]
    pub claims_config: Account<'info, ClaimsConfig>,

    #[account(
        mut,
        seeds = [ClaimAccount::SEED_PREFIX, &claim.claim_id.to_le_bytes()],
        bump = claim.bump,
        constraint = claim.status == ClaimStatus::PendingAttestation @ ClaimsError::InvalidClaimStatus
    )]
    pub claim: Account<'info, ClaimAccount>,

    #[account(
        seeds = [AttestorRegistry::SEED_PREFIX],
        bump = attestor_registry.bump,
        constraint = attestor_registry.is_attestor(&attestor.key()) @ ClaimsError::AttestorNotRegistered
    )]
    pub attestor_registry: Account<'info, AttestorRegistry>,

    #[account(
        init,
        payer = attestor,
        space = 8 + Attestation::INIT_SPACE,
        seeds = [Attestation::SEED_PREFIX, &claim.claim_id.to_le_bytes(), attestor.key().as_ref()],
        bump
    )]
    pub attestation: Account<'info, Attestation>,

    #[account(mut)]
    pub attestor: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AttestClaimParams {
    pub recommendation: AttestationRecommendation,
    pub recommended_amount: u64,
    pub notes_hash: String,
}

pub fn attest_claim(ctx: Context<AttestClaim>, params: AttestClaimParams) -> Result<()> {
    let clock = Clock::get()?;
    let config = &ctx.accounts.claims_config;
    let claim = &mut ctx.accounts.claim;

    // Check attestation is within time limit
    let time_since_submission = clock.unix_timestamp - claim.submitted_at;
    require!(
        time_since_submission <= config.max_attestation_time,
        ClaimsError::AttestationExpired
    );

    // Record attestation
    let attestation = &mut ctx.accounts.attestation;
    attestation.claim_id = claim.claim_id;
    attestation.attestor = ctx.accounts.attestor.key();
    attestation.recommendation = params.recommendation;
    attestation.recommended_amount = params.recommended_amount;
    attestation.notes_hash = params.notes_hash;
    attestation.attested_at = clock.unix_timestamp;
    attestation.bump = ctx.bumps.attestation;

    // Increment attestation count
    claim.attestation_count += 1;

    emit!(ClaimAttested {
        claim_id: claim.claim_id,
        attestor: ctx.accounts.attestor.key(),
        recommendation: params.recommendation,
        recommended_amount: params.recommended_amount,
        attestation_count: claim.attestation_count,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Check attestation status and determine if claim can be resolved
#[derive(Accounts)]
pub struct CheckAttestations<'info> {
    #[account(
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump = claims_config.bump,
    )]
    pub claims_config: Account<'info, ClaimsConfig>,

    #[account(
        seeds = [ClaimAccount::SEED_PREFIX, &claim.claim_id.to_le_bytes()],
        bump = claim.bump,
    )]
    pub claim: Account<'info, ClaimAccount>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AttestationStatus {
    pub total_attestations: u8,
    pub required_attestations: u8,
    pub can_resolve: bool,
    pub time_remaining: i64,
}

pub fn check_attestations(ctx: Context<CheckAttestations>) -> Result<AttestationStatus> {
    let clock = Clock::get()?;
    let config = &ctx.accounts.claims_config;
    let claim = &ctx.accounts.claim;

    let time_elapsed = clock.unix_timestamp - claim.submitted_at;
    let time_remaining = config.max_attestation_time - time_elapsed;
    let can_resolve = claim.attestation_count >= config.required_attestations;

    Ok(AttestationStatus {
        total_attestations: claim.attestation_count,
        required_attestations: config.required_attestations,
        can_resolve,
        time_remaining: time_remaining.max(0),
    })
}
