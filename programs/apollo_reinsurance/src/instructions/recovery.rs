use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::state::{
    ReinsuranceConfig, ReinsuranceTreaty, RecoveryClaim, MemberClaimsAccumulator,
    ReinsuranceLayerType, TreatyStatus, RecoveryStatus
};
use crate::errors::ReinsuranceError;
use crate::events::*;

// ============================================================================
// FILE RECOVERY CLAIM (SPECIFIC STOP-LOSS)
// ============================================================================

#[derive(Accounts)]
#[instruction(params: FileSpecificRecoveryParams)]
pub struct FileSpecificRecovery<'info> {
    #[account(
        mut,
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,
    
    #[account(
        mut,
        constraint = treaty.layer_type == ReinsuranceLayerType::SpecificStopLoss 
            @ ReinsuranceError::TreatyTypeMismatch,
        constraint = treaty.status == TreatyStatus::Active @ ReinsuranceError::TreatyNotActive,
    )]
    pub treaty: Account<'info, ReinsuranceTreaty>,
    
    #[account(
        mut,
        constraint = accumulator.stop_loss_triggered @ ReinsuranceError::StopLossNotTriggered,
    )]
    pub accumulator: Account<'info, MemberClaimsAccumulator>,
    
    #[account(
        init,
        payer = authority,
        space = RecoveryClaim::SIZE,
        seeds = [
            b"recovery_claim",
            treaty.key().as_ref(),
            &(treaty.recovery_claims_count + 1).to_le_bytes()
        ],
        bump
    )]
    pub recovery_claim: Account<'info, RecoveryClaim>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct FileSpecificRecoveryParams {
    /// Member identifier hash (for privacy)
    pub member_hash: [u8; 32],
    
    /// Original claim IDs that triggered this recovery
    pub original_claim_ids: Vec<u64>,
    
    /// Supporting documentation hash
    pub documentation_hash: [u8; 32],
}

pub fn file_specific_recovery(
    ctx: Context<FileSpecificRecovery>,
    params: FileSpecificRecoveryParams,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let treaty = &mut ctx.accounts.treaty;
    let accumulator = &ctx.accounts.accumulator;
    let claim = &mut ctx.accounts.recovery_claim;
    let clock = Clock::get()?;
    
    // Verify treaty is within its effective period
    require!(
        treaty.is_active(clock.unix_timestamp),
        ReinsuranceError::TreatyNotActive
    );
    
    // Calculate excess amount (claims above attachment)
    let excess_amount = accumulator.ytd_claims
        .saturating_sub(treaty.attachment_point);
    
    require!(excess_amount > 0, ReinsuranceError::NoExcessAmount);
    
    // Calculate coverage split
    let (apollo_portion, reinsurer_portion) = treaty.calculate_coverage(excess_amount);
    
    // Increment counters
    config.total_recovery_claims = config.total_recovery_claims.checked_add(1)
        .ok_or(ReinsuranceError::Overflow)?;
    
    treaty.recovery_claims_count = treaty.recovery_claims_count.checked_add(1)
        .ok_or(ReinsuranceError::Overflow)?;
    treaty.claims_pending_count = treaty.claims_pending_count.checked_add(1)
        .ok_or(ReinsuranceError::Overflow)?;
    treaty.last_updated = clock.unix_timestamp;
    
    // Track pending recoveries
    config.pending_recoveries = config.pending_recoveries.checked_add(reinsurer_portion)
        .ok_or(ReinsuranceError::Overflow)?;
    
    // Initialize claim
    claim.claim_id = config.total_recovery_claims;
    claim.treaty = treaty.key();
    claim.layer_type = ReinsuranceLayerType::SpecificStopLoss;
    claim.status = RecoveryStatus::Pending;
    claim.member_hash = params.member_hash;
    
    // Copy claim IDs (up to 8)
    let count = params.original_claim_ids.len().min(8);
    for i in 0..count {
        claim.original_claim_ids[i] = params.original_claim_ids[i];
    }
    claim.original_claims_count = count as u8;
    
    claim.total_claims_amount = accumulator.ytd_claims;
    claim.attachment_point = treaty.attachment_point;
    claim.excess_amount = excess_amount;
    claim.apollo_portion = apollo_portion;
    claim.claimed_amount = reinsurer_portion;
    
    claim.event_timestamp = accumulator.first_trigger_timestamp;
    claim.filed_timestamp = clock.unix_timestamp;
    claim.filed_by = ctx.accounts.authority.key();
    claim.documentation_hash = params.documentation_hash;
    claim.bump = ctx.bumps.recovery_claim;
    
    emit!(RecoveryClaimFiled {
        claim_id: claim.claim_id,
        claim_pubkey: claim.key(),
        treaty_id: treaty.treaty_id,
        layer_type: claim.layer_type,
        total_claims_amount: claim.total_claims_amount,
        excess_amount: claim.excess_amount,
        claimed_amount: claim.claimed_amount,
        filed_by: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });
    
    emit!(SpecificStopLossTriggered {
        member: accumulator.member,
        member_hash: params.member_hash,
        treaty_id: treaty.treaty_id,
        total_claims: accumulator.ytd_claims,
        attachment_point: treaty.attachment_point,
        excess_amount,
        apollo_portion,
        reinsurer_portion,
        triggering_claim_id: params.original_claim_ids.first().copied().unwrap_or(0),
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Recovery claim {} filed: {} USDC recoverable",
        claim.claim_id,
        reinsurer_portion / 1_000_000
    );
    
    Ok(())
}

// ============================================================================
// FILE AGGREGATE RECOVERY
// ============================================================================

#[derive(Accounts)]
pub struct FileAggregateRecovery<'info> {
    #[account(
        mut,
        seeds = [b"reinsurance_config"],
        bump = config.bump,
        constraint = config.aggregate_triggered @ ReinsuranceError::AggregateNotTriggered,
    )]
    pub config: Account<'info, ReinsuranceConfig>,
    
    #[account(
        mut,
        constraint = treaty.layer_type == ReinsuranceLayerType::AggregateStopLoss 
            @ ReinsuranceError::TreatyTypeMismatch,
        constraint = treaty.status == TreatyStatus::Active @ ReinsuranceError::TreatyNotActive,
    )]
    pub treaty: Account<'info, ReinsuranceTreaty>,
    
    #[account(
        init,
        payer = authority,
        space = RecoveryClaim::SIZE,
        seeds = [
            b"recovery_claim",
            treaty.key().as_ref(),
            &(treaty.recovery_claims_count + 1).to_le_bytes()
        ],
        bump
    )]
    pub recovery_claim: Account<'info, RecoveryClaim>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn file_aggregate_recovery(
    ctx: Context<FileAggregateRecovery>,
    documentation_hash: [u8; 32],
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let treaty = &mut ctx.accounts.treaty;
    let claim = &mut ctx.accounts.recovery_claim;
    let clock = Clock::get()?;
    
    // Calculate recoverable amount
    let recoverable = config.calculate_aggregate_recoverable();
    require!(recoverable > 0, ReinsuranceError::NoExcessAmount);
    
    // Calculate trigger threshold in USDC
    let trigger_amount = config.expected_annual_claims
        .checked_mul(treaty.trigger_ratio_bps as u64)
        .ok_or(ReinsuranceError::Overflow)?
        .checked_div(10_000)
        .ok_or(ReinsuranceError::DivisionByZero)?;
    
    // For aggregate, typically 100% coverage (0 coinsurance)
    let (apollo_portion, reinsurer_portion) = treaty.calculate_coverage(recoverable);
    
    // Update counters
    config.total_recovery_claims = config.total_recovery_claims.checked_add(1)
        .ok_or(ReinsuranceError::Overflow)?;
    
    treaty.recovery_claims_count = treaty.recovery_claims_count.checked_add(1)
        .ok_or(ReinsuranceError::Overflow)?;
    treaty.claims_pending_count = treaty.claims_pending_count.checked_add(1)
        .ok_or(ReinsuranceError::Overflow)?;
    treaty.last_updated = clock.unix_timestamp;
    
    config.pending_recoveries = config.pending_recoveries.checked_add(reinsurer_portion)
        .ok_or(ReinsuranceError::Overflow)?;
    
    // Initialize claim
    claim.claim_id = config.total_recovery_claims;
    claim.treaty = treaty.key();
    claim.layer_type = ReinsuranceLayerType::AggregateStopLoss;
    claim.status = RecoveryStatus::Pending;
    
    claim.total_claims_amount = config.ytd_claims_paid;
    claim.attachment_point = trigger_amount;
    claim.excess_amount = recoverable;
    claim.apollo_portion = apollo_portion;
    claim.claimed_amount = reinsurer_portion;
    
    claim.filed_timestamp = clock.unix_timestamp;
    claim.filed_by = ctx.accounts.authority.key();
    claim.documentation_hash = documentation_hash;
    claim.bump = ctx.bumps.recovery_claim;
    
    emit!(RecoveryClaimFiled {
        claim_id: claim.claim_id,
        claim_pubkey: claim.key(),
        treaty_id: treaty.treaty_id,
        layer_type: claim.layer_type,
        total_claims_amount: claim.total_claims_amount,
        excess_amount: claim.excess_amount,
        claimed_amount: claim.claimed_amount,
        filed_by: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Aggregate recovery claim {} filed: {} USDC recoverable",
        claim.claim_id,
        reinsurer_portion / 1_000_000
    );
    
    Ok(())
}

// ============================================================================
// SUBMIT CLAIM TO REINSURER
// ============================================================================

#[derive(Accounts)]
pub struct SubmitRecoveryToReinsurer<'info> {
    #[account(
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,
    
    #[account(mut)]
    pub treaty: Account<'info, ReinsuranceTreaty>,
    
    #[account(
        mut,
        constraint = recovery_claim.status == RecoveryStatus::Pending 
            @ ReinsuranceError::InvalidRecoveryStatus,
        constraint = recovery_claim.treaty == treaty.key() @ ReinsuranceError::TreatyTypeMismatch,
    )]
    pub recovery_claim: Account<'info, RecoveryClaim>,
    
    pub authority: Signer<'info>,
}

pub fn submit_recovery_to_reinsurer(
    ctx: Context<SubmitRecoveryToReinsurer>,
    documentation_hash: [u8; 32],
) -> Result<()> {
    let config = &ctx.accounts.config;
    let treaty = &mut ctx.accounts.treaty;
    let claim = &mut ctx.accounts.recovery_claim;
    let clock = Clock::get()?;
    
    // Validate authority
    let is_authorized = ctx.accounts.authority.key() == config.authority
        || ctx.accounts.authority.key() == config.reinsurance_committee;
    require!(is_authorized, ReinsuranceError::Unauthorized);
    
    claim.status = RecoveryStatus::Submitted;
    claim.submitted_timestamp = clock.unix_timestamp;
    claim.documentation_hash = documentation_hash;
    
    treaty.total_claims_submitted = treaty.total_claims_submitted
        .checked_add(claim.claimed_amount)
        .ok_or(ReinsuranceError::Overflow)?;
    treaty.last_updated = clock.unix_timestamp;
    
    emit!(RecoveryClaimSubmitted {
        claim_id: claim.claim_id,
        treaty_id: treaty.treaty_id,
        claimed_amount: claim.claimed_amount,
        documentation_hash,
        submitted_by: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Recovery claim {} submitted to reinsurer", claim.claim_id);
    
    Ok(())
}

// ============================================================================
// RECORD REINSURER DECISION
// ============================================================================

#[derive(Accounts)]
pub struct RecordReinsurerDecision<'info> {
    #[account(
        mut,
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,
    
    #[account(mut)]
    pub treaty: Account<'info, ReinsuranceTreaty>,
    
    #[account(
        mut,
        constraint = recovery_claim.status == RecoveryStatus::Submitted 
            || recovery_claim.status == RecoveryStatus::UnderReview
            @ ReinsuranceError::InvalidRecoveryStatus,
    )]
    pub recovery_claim: Account<'info, RecoveryClaim>,
    
    pub authority: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum ReinsurerDecision {
    /// Claim approved for the specified amount
    Approved { amount: u64, reference: [u8; 32] },
    
    /// Claim denied
    Denied { reason_hash: [u8; 32] },
    
    /// Claim disputed - partial approval or negotiation needed
    Disputed { disputed_amount: u64, reason_hash: [u8; 32] },
    
    /// Claim under review - needs more time
    UnderReview,
}

pub fn record_reinsurer_decision(
    ctx: Context<RecordReinsurerDecision>,
    decision: ReinsurerDecision,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let treaty = &mut ctx.accounts.treaty;
    let claim = &mut ctx.accounts.recovery_claim;
    let clock = Clock::get()?;
    
    // Validate authority
    let is_authorized = ctx.accounts.authority.key() == config.authority
        || ctx.accounts.authority.key() == config.reinsurance_committee;
    require!(is_authorized, ReinsuranceError::Unauthorized);
    
    match decision {
        ReinsurerDecision::Approved { amount, reference } => {
            require!(
                amount <= claim.claimed_amount,
                ReinsuranceError::RecoveryExceedsClaim
            );
            
            claim.status = RecoveryStatus::Approved;
            claim.approved_amount = amount;
            claim.reinsurer_reference = reference;
            claim.resolution_timestamp = clock.unix_timestamp;
            
            emit!(RecoveryClaimApproved {
                claim_id: claim.claim_id,
                treaty_id: treaty.treaty_id,
                claimed_amount: claim.claimed_amount,
                approved_amount: amount,
                reinsurer_reference: reference,
                timestamp: clock.unix_timestamp,
            });
            
            msg!("Recovery claim {} approved: {} USDC", claim.claim_id, amount / 1_000_000);
        }
        
        ReinsurerDecision::Denied { reason_hash } => {
            claim.status = RecoveryStatus::Denied;
            claim.approved_amount = 0;
            claim.resolution_notes_hash = reason_hash;
            claim.resolution_timestamp = clock.unix_timestamp;
            
            // Update pending recoveries
            config.pending_recoveries = config.pending_recoveries
                .saturating_sub(claim.claimed_amount);
            
            treaty.claims_pending_count = treaty.claims_pending_count.saturating_sub(1);
            
            emit!(RecoveryClaimDenied {
                claim_id: claim.claim_id,
                treaty_id: treaty.treaty_id,
                claimed_amount: claim.claimed_amount,
                reason_hash,
                timestamp: clock.unix_timestamp,
            });
            
            msg!("Recovery claim {} denied", claim.claim_id);
        }
        
        ReinsurerDecision::Disputed { disputed_amount, reason_hash } => {
            claim.status = RecoveryStatus::Disputed;
            claim.resolution_notes_hash = reason_hash;
            
            emit!(RecoveryClaimDisputed {
                claim_id: claim.claim_id,
                treaty_id: treaty.treaty_id,
                claimed_amount: claim.claimed_amount,
                disputed_amount,
                reason_hash,
                timestamp: clock.unix_timestamp,
            });
            
            msg!("Recovery claim {} disputed: {} USDC in dispute", 
                claim.claim_id, 
                disputed_amount / 1_000_000
            );
        }
        
        ReinsurerDecision::UnderReview => {
            claim.status = RecoveryStatus::UnderReview;
            msg!("Recovery claim {} under review", claim.claim_id);
        }
    }
    
    treaty.last_updated = clock.unix_timestamp;
    
    Ok(())
}

// ============================================================================
// RECORD SETTLEMENT (PAYMENT RECEIVED)
// ============================================================================

#[derive(Accounts)]
pub struct RecordSettlement<'info> {
    #[account(
        mut,
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,
    
    #[account(mut)]
    pub treaty: Account<'info, ReinsuranceTreaty>,
    
    #[account(
        mut,
        constraint = recovery_claim.status == RecoveryStatus::Approved 
            || recovery_claim.status == RecoveryStatus::Disputed
            @ ReinsuranceError::InvalidRecoveryStatus,
    )]
    pub recovery_claim: Account<'info, RecoveryClaim>,
    
    /// Where the reinsurer payment was received
    #[account(mut)]
    pub settlement_account: Account<'info, TokenAccount>,
    
    pub authority: Signer<'info>,
}

pub fn record_settlement(
    ctx: Context<RecordSettlement>,
    received_amount: u64,
    is_final: bool,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let treaty = &mut ctx.accounts.treaty;
    let claim = &mut ctx.accounts.recovery_claim;
    let clock = Clock::get()?;
    
    // Validate authority
    let is_authorized = ctx.accounts.authority.key() == config.authority
        || ctx.accounts.authority.key() == config.reinsurance_committee;
    require!(is_authorized, ReinsuranceError::Unauthorized);
    
    // Update claim
    claim.received_amount = claim.received_amount.checked_add(received_amount)
        .ok_or(ReinsuranceError::Overflow)?;
    claim.settlement_timestamp = clock.unix_timestamp;
    
    if is_final {
        if claim.received_amount >= claim.approved_amount {
            claim.status = RecoveryStatus::Settled;
            treaty.claims_settled_count = treaty.claims_settled_count.checked_add(1)
                .ok_or(ReinsuranceError::Overflow)?;
        } else {
            claim.status = RecoveryStatus::PartiallySettled;
        }
        treaty.claims_pending_count = treaty.claims_pending_count.saturating_sub(1);
    }
    
    // Update global tracking
    config.ytd_recoveries_received = config.ytd_recoveries_received
        .checked_add(received_amount)
        .ok_or(ReinsuranceError::Overflow)?;
    
    config.pending_recoveries = config.pending_recoveries
        .saturating_sub(received_amount.min(claim.claimed_amount));
    
    treaty.total_recoveries_received = treaty.total_recoveries_received
        .checked_add(received_amount)
        .ok_or(ReinsuranceError::Overflow)?;
    treaty.last_updated = clock.unix_timestamp;
    
    emit!(RecoverySettled {
        claim_id: claim.claim_id,
        treaty_id: treaty.treaty_id,
        claimed_amount: claim.claimed_amount,
        approved_amount: claim.approved_amount,
        received_amount: claim.received_amount,
        is_partial: claim.status == RecoveryStatus::PartiallySettled,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Recovery claim {} settled: {} USDC received",
        claim.claim_id,
        received_amount / 1_000_000
    );
    
    Ok(())
}
