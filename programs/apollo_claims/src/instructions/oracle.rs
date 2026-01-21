// programs/apollo_claims/src/instructions/oracle.rs
//
// AI Claims Oracle Instructions
// Tier 2 of three-tier claims processing
// ML-assisted triage for medium complexity claims

use anchor_lang::prelude::*;
use crate::ai_oracle::{
    ClaimsOracle, AiDecision, AiDecisionType, AiFlag, UcrReference,
    RegionalPriceFactor, AiDecisionSubmitted, AiDecisionOverturned,
};
use crate::state::{ClaimsConfig, ClaimAccount, ClaimStatus};
use crate::errors::ClaimsError;

// =============================================================================
// INITIALIZATION
// =============================================================================

#[derive(Accounts)]
pub struct InitializeClaimsOracle<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + ClaimsOracle::INIT_SPACE,
        seeds = [ClaimsOracle::SEED_PREFIX],
        bump
    )]
    pub claims_oracle: Account<'info, ClaimsOracle>,

    #[account(
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump = claims_config.bump,
        constraint = claims_config.authority == authority.key() @ ClaimsError::Unauthorized
    )]
    pub claims_config: Account<'info, ClaimsConfig>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeOracleParams {
    /// Initial authorized oracle signers
    pub authorized_signers: Vec<Pubkey>,
    /// Required signatures for AI decision
    pub required_sigs: u8,
    /// Confidence threshold for auto-approve (bps)
    pub auto_approve_confidence_bps: u16,
    /// Confidence threshold for auto-deny (bps)
    pub auto_deny_confidence_bps: u16,
    /// Max fraud score for auto-approve (bps)
    pub max_fraud_score_for_approve_bps: u16,
}

pub fn initialize_claims_oracle(
    ctx: Context<InitializeClaimsOracle>,
    params: InitializeOracleParams,
) -> Result<()> {
    let oracle = &mut ctx.accounts.claims_oracle;
    
    require!(
        params.authorized_signers.len() >= params.required_sigs as usize,
        ClaimsError::InsufficientSigners
    );
    
    oracle.authority = ctx.accounts.authority.key();
    oracle.authorized_signers = params.authorized_signers;
    oracle.required_sigs = params.required_sigs;
    oracle.total_decisions = 0;
    oracle.auto_approved = 0;
    oracle.auto_denied = 0;
    oracle.escalated = 0;
    oracle.decisions_overturned = 0;
    oracle.accuracy_rate_bps = 10000; // Start at 100%
    oracle.auto_approve_confidence_bps = params.auto_approve_confidence_bps;
    oracle.auto_deny_confidence_bps = params.auto_deny_confidence_bps;
    oracle.max_fraud_score_for_approve_bps = params.max_fraud_score_for_approve_bps;
    oracle.is_active = true;
    oracle.last_decision_at = 0;
    oracle.bump = ctx.bumps.claims_oracle;
    
    Ok(())
}

// =============================================================================
// SUBMIT AI DECISION
// =============================================================================

#[derive(Accounts)]
#[instruction(params: AiDecisionParams)]
pub struct SubmitAiDecision<'info> {
    #[account(
        mut,
        seeds = [ClaimsOracle::SEED_PREFIX],
        bump = claims_oracle.bump,
        constraint = claims_oracle.is_active @ ClaimsError::OracleDisabled,
        constraint = claims_oracle.is_authorized(&oracle_signer.key()) @ ClaimsError::Unauthorized
    )]
    pub claims_oracle: Account<'info, ClaimsOracle>,

    #[account(
        init,
        payer = payer,
        space = 8 + AiDecision::INIT_SPACE,
        seeds = [AiDecision::SEED_PREFIX, &params.claim_id.to_le_bytes()],
        bump
    )]
    pub ai_decision: Account<'info, AiDecision>,

    #[account(
        mut,
        seeds = [ClaimAccount::SEED_PREFIX, &params.claim_id.to_le_bytes()],
        bump = claim.bump,
        constraint = claim.status == ClaimStatus::UnderReview @ ClaimsError::InvalidClaimStatus
    )]
    pub claim: Account<'info, ClaimAccount>,

    #[account(
        mut,
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump = claims_config.bump,
    )]
    pub claims_config: Account<'info, ClaimsConfig>,

    /// Oracle service signer (must be authorized)
    pub oracle_signer: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AiDecisionParams {
    pub claim_id: u64,
    pub decision: AiDecisionType,
    pub confidence_bps: u16,
    pub price_score_bps: u16,
    pub fraud_score_bps: u16,
    pub consistency_score_bps: u16,
    pub flags: Vec<AiFlag>,
    pub suggested_amount: u64,
    pub reference_price: u64,
}

pub fn submit_ai_decision(
    ctx: Context<SubmitAiDecision>,
    params: AiDecisionParams,
) -> Result<()> {
    let clock = Clock::get()?;
    let oracle = &mut ctx.accounts.claims_oracle;
    let ai_decision = &mut ctx.accounts.ai_decision;
    let claim = &mut ctx.accounts.claim;
    let config = &mut ctx.accounts.claims_config;
    
    // Record the AI decision
    ai_decision.claim_id = params.claim_id;
    ai_decision.decision = params.decision;
    ai_decision.confidence_bps = params.confidence_bps;
    ai_decision.price_score_bps = params.price_score_bps;
    ai_decision.fraud_score_bps = params.fraud_score_bps;
    ai_decision.consistency_score_bps = params.consistency_score_bps;
    ai_decision.flags = params.flags.clone();
    ai_decision.suggested_amount = params.suggested_amount;
    ai_decision.reference_price = params.reference_price;
    ai_decision.submitted_by = ctx.accounts.oracle_signer.key();
    ai_decision.submitted_at = clock.unix_timestamp;
    ai_decision.overturned = false;
    ai_decision.overturn_reason = String::new();
    ai_decision.bump = ctx.bumps.ai_decision;
    
    // Update oracle stats
    oracle.total_decisions += 1;
    oracle.last_decision_at = clock.unix_timestamp;
    
    // Execute the decision on the claim
    match params.decision {
        AiDecisionType::AutoApprove => {
            claim.status = ClaimStatus::Approved;
            claim.approved_amount = params.suggested_amount;
            oracle.auto_approved += 1;
            config.total_claims_approved += 1;
        },
        AiDecisionType::AutoDeny => {
            claim.status = ClaimStatus::Denied;
            claim.denial_reason = format!("AI denial: High fraud risk ({}bps)", params.fraud_score_bps);
            oracle.auto_denied += 1;
            config.total_claims_denied += 1;
        },
        AiDecisionType::CommitteeReview => {
            claim.status = ClaimStatus::PendingAttestation;
            oracle.escalated += 1;
        },
        AiDecisionType::RequestInfo => {
            // Keep in UnderReview, member needs to provide more info
            // In production, this would trigger a notification
            oracle.escalated += 1;
        },
    }
    
    claim.status_changed_at = clock.unix_timestamp;
    
    emit!(AiDecisionSubmitted {
        claim_id: params.claim_id,
        decision: params.decision,
        confidence_bps: params.confidence_bps,
        fraud_score_bps: params.fraud_score_bps,
        flags_count: params.flags.len() as u8,
        oracle_signer: ctx.accounts.oracle_signer.key(),
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}

// =============================================================================
// OVERTURN AI DECISION
// =============================================================================

#[derive(Accounts)]
pub struct OverturnAiDecision<'info> {
    #[account(
        mut,
        seeds = [ClaimsOracle::SEED_PREFIX],
        bump = claims_oracle.bump,
    )]
    pub claims_oracle: Account<'info, ClaimsOracle>,

    #[account(
        mut,
        seeds = [AiDecision::SEED_PREFIX, &ai_decision.claim_id.to_le_bytes()],
        bump = ai_decision.bump,
        constraint = !ai_decision.overturned @ ClaimsError::AlreadyOverturned
    )]
    pub ai_decision: Account<'info, AiDecision>,

    #[account(
        mut,
        seeds = [ClaimAccount::SEED_PREFIX, &ai_decision.claim_id.to_le_bytes()],
        bump = claim.bump,
    )]
    pub claim: Account<'info, ClaimAccount>,

    #[account(
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump = claims_config.bump,
        constraint = claims_config.authority == authority.key() ||
                     claims_config.claims_committee == authority.key() @ ClaimsError::Unauthorized
    )]
    pub claims_config: Account<'info, ClaimsConfig>,

    pub authority: Signer<'info>,
}

pub fn overturn_ai_decision(
    ctx: Context<OverturnAiDecision>,
    new_decision: AiDecisionType,
    reason: String,
) -> Result<()> {
    let clock = Clock::get()?;
    let oracle = &mut ctx.accounts.claims_oracle;
    let ai_decision = &mut ctx.accounts.ai_decision;
    let claim = &mut ctx.accounts.claim;
    
    let original_decision = ai_decision.decision;
    
    // Mark as overturned
    ai_decision.overturned = true;
    ai_decision.overturn_reason = reason.clone();
    
    // Update oracle stats
    oracle.decisions_overturned += 1;
    oracle.update_accuracy();
    
    // Apply new decision to claim
    match new_decision {
        AiDecisionType::AutoApprove => {
            claim.status = ClaimStatus::Approved;
            claim.approved_amount = ai_decision.suggested_amount;
        },
        AiDecisionType::AutoDeny => {
            claim.status = ClaimStatus::Denied;
            claim.denial_reason = reason.clone();
        },
        AiDecisionType::CommitteeReview => {
            claim.status = ClaimStatus::PendingAttestation;
        },
        AiDecisionType::RequestInfo => {
            claim.status = ClaimStatus::UnderReview;
        },
    }
    
    claim.status_changed_at = clock.unix_timestamp;
    
    emit!(AiDecisionOverturned {
        claim_id: ai_decision.claim_id,
        original_decision,
        new_decision,
        overturn_reason: reason,
        overturned_by: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}

// =============================================================================
// UCR PRICE REFERENCE
// =============================================================================

#[derive(Accounts)]
#[instruction(params: UcrUpdateParams)]
pub struct UpdateUcrReference<'info> {
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + UcrReference::INIT_SPACE,
        seeds = [UcrReference::SEED_PREFIX, params.procedure_code.as_bytes()],
        bump
    )]
    pub ucr_reference: Account<'info, UcrReference>,

    #[account(
        seeds = [ClaimsOracle::SEED_PREFIX],
        bump = claims_oracle.bump,
        constraint = claims_oracle.is_authorized(&authority.key()) ||
                     claims_oracle.authority == authority.key() @ ClaimsError::Unauthorized
    )]
    pub claims_oracle: Account<'info, ClaimsOracle>,

    pub authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UcrUpdateParams {
    pub procedure_code: String,
    pub base_price: u64,
    pub price_low: u64,
    pub price_high: u64,
    pub regional_factors: Vec<RegionalPriceFactor>,
    pub source_hash: String,
}

pub fn update_ucr_reference(
    ctx: Context<UpdateUcrReference>,
    params: UcrUpdateParams,
) -> Result<()> {
    let clock = Clock::get()?;
    let ucr = &mut ctx.accounts.ucr_reference;
    
    ucr.procedure_code = params.procedure_code;
    ucr.base_price = params.base_price;
    ucr.price_low = params.price_low;
    ucr.price_high = params.price_high;
    ucr.regional_factors = params.regional_factors;
    ucr.last_updated = clock.unix_timestamp;
    ucr.source_hash = params.source_hash;
    ucr.bump = ctx.bumps.ucr_reference;
    
    Ok(())
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Check if oracle has sufficient authorized signers
pub fn validate_oracle_config(oracle: &ClaimsOracle) -> Result<()> {
    require!(
        oracle.authorized_signers.len() >= oracle.required_sigs as usize,
        ClaimsError::InsufficientSigners
    );
    require!(oracle.is_active, ClaimsError::OracleDisabled);
    Ok(())
}
