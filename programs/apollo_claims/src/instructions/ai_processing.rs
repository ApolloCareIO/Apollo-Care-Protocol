// programs/apollo_claims/src/instructions/ai_processing.rs
//
// AI/ML-driven claims processing for the three-tier system:
// 1. Fast-Lane Auto-Approval (small routine claims)
// 2. AI-Assisted Triage (medium claims, fraud detection)
// 3. Committee Escalation (large/complex claims)

use anchor_lang::prelude::*;
use crate::state::{ClaimsConfig, ClaimAccount, ClaimStatus, ClaimCategory};
use crate::errors::ClaimsError;
use crate::events::ClaimStatusChanged;

// =============================================================================
// AI ORACLE STATE
// =============================================================================

/// AI Oracle configuration - manages off-chain ML decision system
/// PDA seeds: ["ai_oracle"]
#[account]
#[derive(InitSpace)]
pub struct AiOracle {
    /// Authority (DAO or Actuarial Committee)
    pub authority: Pubkey,
    
    /// List of authorized oracle signers (can submit AI decisions)
    #[max_len(5)]
    pub authorized_signers: Vec<Pubkey>,
    
    /// Required signatures for AI decision to be accepted
    pub required_sigs: u8,
    
    /// Total decisions processed
    pub total_decisions: u64,
    
    /// Decisions that were auto-approved
    pub auto_approved: u64,
    
    /// Decisions that were auto-denied
    pub auto_denied: u64,
    
    /// Decisions escalated to committee
    pub escalated_to_committee: u64,
    
    /// Decisions overturned by committee (accuracy tracking)
    pub decisions_overturned: u64,
    
    /// Oracle accuracy rate (bps) - updated periodically
    pub accuracy_rate_bps: u16,
    
    /// Minimum confidence for auto-approval (bps, e.g., 9500 = 95%)
    pub min_auto_approve_confidence_bps: u16,
    
    /// Maximum fraud score for auto-approval (bps, e.g., 3000 = 30%)
    pub max_fraud_score_for_approval_bps: u16,
    
    /// Minimum confidence for any decision (below this = committee)
    pub min_confidence_threshold_bps: u16,
    
    /// Is oracle active
    pub is_active: bool,
    
    /// Bump seed
    pub bump: u8,
}

impl AiOracle {
    pub const SEED_PREFIX: &'static [u8] = b"ai_oracle";
    
    // Bootstrap defaults - more conservative
    pub const DEFAULT_MIN_AUTO_APPROVE_CONFIDENCE: u16 = 9500; // 95%
    pub const DEFAULT_MAX_FRAUD_SCORE: u16 = 3000; // 30%
    pub const DEFAULT_MIN_CONFIDENCE: u16 = 7000; // 70%
    
    pub fn is_authorized_signer(&self, signer: &Pubkey) -> bool {
        self.authorized_signers.contains(signer)
    }
}

/// AI Decision record for a specific claim
/// PDA seeds: ["ai_decision", claim_id]
#[account]
#[derive(InitSpace)]
pub struct AiDecision {
    /// Claim this decision is for
    pub claim_id: u64,
    
    /// Decision type
    pub decision: AiDecisionType,
    
    /// Overall confidence score (bps)
    pub confidence_bps: u16,
    
    /// Price reasonableness score (bps, 10000 = perfectly reasonable)
    pub price_score_bps: u16,
    
    /// Fraud risk score (bps, 0 = no risk, 10000 = certain fraud)
    pub fraud_score_bps: u16,
    
    /// Procedure-diagnosis consistency score (bps)
    pub consistency_score_bps: u16,
    
    /// Suggested approved amount (may differ from requested)
    pub suggested_amount: u64,
    
    /// Flags raised by AI (up to 5)
    #[max_len(5, 64)]
    pub flags: Vec<String>,
    
    /// Oracle signer who submitted this decision
    pub submitted_by: Pubkey,
    
    /// Timestamp of decision
    pub decided_at: i64,
    
    /// Was this decision overturned by committee?
    pub overturned: bool,
    
    /// Bump seed
    pub bump: u8,
}

impl AiDecision {
    pub const SEED_PREFIX: &'static [u8] = b"ai_decision";
}

/// AI decision types
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq, InitSpace)]
pub enum AiDecisionType {
    /// Auto-approve the claim
    AutoApprove,
    /// Auto-deny with reason
    AutoDeny {
        #[max_len(128)]
        reason: String,
    },
    /// Escalate to committee for human review
    CommitteeReview,
}

impl Default for AiDecisionType {
    fn default() -> Self {
        AiDecisionType::CommitteeReview
    }
}

// =============================================================================
// FAST-LANE AUTO-APPROVAL
// =============================================================================

/// Fast-lane eligibility criteria
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct FastLaneConfig {
    /// Maximum claim amount for fast-lane (USDC)
    pub max_amount: u64,
    /// Eligible categories
    pub eligible_categories: Vec<ClaimCategory>,
    /// Maximum fast-lane claims per member per month
    pub max_per_member_per_month: u8,
}

impl Default for FastLaneConfig {
    fn default() -> Self {
        Self {
            max_amount: 500_000_000, // $500 bootstrap
            eligible_categories: vec![
                ClaimCategory::PrimaryCare,
                ClaimCategory::Prescription,
                ClaimCategory::Laboratory,
                ClaimCategory::Preventive,
                ClaimCategory::DiagnosticImaging,
            ],
            max_per_member_per_month: 5,
        }
    }
}

/// Member's fast-lane usage tracking
/// PDA seeds: ["fast_lane_usage", member, month_timestamp]
#[account]
#[derive(InitSpace)]
pub struct FastLaneUsage {
    /// Member pubkey
    pub member: Pubkey,
    /// Month start timestamp (first of month)
    pub month_start: i64,
    /// Number of fast-lane claims used this month
    pub claims_used: u8,
    /// Total amount claimed via fast-lane this month
    pub amount_claimed: u64,
    /// Bump seed
    pub bump: u8,
}

impl FastLaneUsage {
    pub const SEED_PREFIX: &'static [u8] = b"fast_lane_usage";
}

// =============================================================================
// INSTRUCTIONS
// =============================================================================

/// Initialize AI Oracle
#[derive(Accounts)]
pub struct InitializeAiOracle<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + AiOracle::INIT_SPACE,
        seeds = [AiOracle::SEED_PREFIX],
        bump
    )]
    pub ai_oracle: Account<'info, AiOracle>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn initialize_ai_oracle(
    ctx: Context<InitializeAiOracle>,
    authorized_signers: Vec<Pubkey>,
    required_sigs: u8,
) -> Result<()> {
    require!(authorized_signers.len() >= required_sigs as usize, ClaimsError::InvalidConfiguration);
    require!(required_sigs > 0, ClaimsError::InvalidConfiguration);
    
    let oracle = &mut ctx.accounts.ai_oracle;
    oracle.authority = ctx.accounts.authority.key();
    oracle.authorized_signers = authorized_signers;
    oracle.required_sigs = required_sigs;
    oracle.total_decisions = 0;
    oracle.auto_approved = 0;
    oracle.auto_denied = 0;
    oracle.escalated_to_committee = 0;
    oracle.decisions_overturned = 0;
    oracle.accuracy_rate_bps = 10000; // Start at 100%
    oracle.min_auto_approve_confidence_bps = AiOracle::DEFAULT_MIN_AUTO_APPROVE_CONFIDENCE;
    oracle.max_fraud_score_for_approval_bps = AiOracle::DEFAULT_MAX_FRAUD_SCORE;
    oracle.min_confidence_threshold_bps = AiOracle::DEFAULT_MIN_CONFIDENCE;
    oracle.is_active = true;
    oracle.bump = ctx.bumps.ai_oracle;
    
    Ok(())
}

/// Submit AI decision for a claim
#[derive(Accounts)]
#[instruction(claim_id: u64)]
pub struct SubmitAiDecision<'info> {
    #[account(
        seeds = [AiOracle::SEED_PREFIX],
        bump = ai_oracle.bump,
        constraint = ai_oracle.is_active @ ClaimsError::OracleInactive,
        constraint = ai_oracle.is_authorized_signer(&oracle_signer.key()) @ ClaimsError::Unauthorized
    )]
    pub ai_oracle: Account<'info, AiOracle>,
    
    #[account(
        mut,
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump = claims_config.bump,
    )]
    pub claims_config: Account<'info, ClaimsConfig>,
    
    #[account(
        mut,
        seeds = [ClaimAccount::SEED_PREFIX, &claim_id.to_le_bytes()],
        bump = claim.bump,
        constraint = claim.status == ClaimStatus::UnderReview @ ClaimsError::InvalidClaimStatus
    )]
    pub claim: Account<'info, ClaimAccount>,
    
    #[account(
        init,
        payer = oracle_signer,
        space = 8 + AiDecision::INIT_SPACE,
        seeds = [AiDecision::SEED_PREFIX, &claim_id.to_le_bytes()],
        bump
    )]
    pub ai_decision: Account<'info, AiDecision>,
    
    #[account(mut)]
    pub oracle_signer: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AiDecisionParams {
    pub claim_id: u64,
    pub confidence_bps: u16,
    pub price_score_bps: u16,
    pub fraud_score_bps: u16,
    pub consistency_score_bps: u16,
    pub suggested_amount: u64,
    pub flags: Vec<String>,
}

pub fn submit_ai_decision(
    ctx: Context<SubmitAiDecision>,
    params: AiDecisionParams,
) -> Result<()> {
    let clock = Clock::get()?;
    let oracle = &ctx.accounts.ai_oracle;
    let claim = &mut ctx.accounts.claim;
    let ai_decision = &mut ctx.accounts.ai_decision;
    
    // Determine decision based on scores and thresholds
    let decision = determine_ai_decision(
        params.confidence_bps,
        params.fraud_score_bps,
        oracle.min_auto_approve_confidence_bps,
        oracle.max_fraud_score_for_approval_bps,
        oracle.min_confidence_threshold_bps,
        &params.flags,
    );
    
    // Record decision
    ai_decision.claim_id = params.claim_id;
    ai_decision.decision = decision.clone();
    ai_decision.confidence_bps = params.confidence_bps;
    ai_decision.price_score_bps = params.price_score_bps;
    ai_decision.fraud_score_bps = params.fraud_score_bps;
    ai_decision.consistency_score_bps = params.consistency_score_bps;
    ai_decision.suggested_amount = params.suggested_amount;
    ai_decision.flags = params.flags;
    ai_decision.submitted_by = ctx.accounts.oracle_signer.key();
    ai_decision.decided_at = clock.unix_timestamp;
    ai_decision.overturned = false;
    ai_decision.bump = ctx.bumps.ai_decision;
    
    // Update claim status based on decision
    let old_status = claim.status;
    match &decision {
        AiDecisionType::AutoApprove => {
            claim.status = ClaimStatus::Approved;
            claim.approved_amount = params.suggested_amount;
        },
        AiDecisionType::AutoDeny { .. } => {
            claim.status = ClaimStatus::Denied;
            // denial_reason would be set from the decision
        },
        AiDecisionType::CommitteeReview => {
            claim.status = ClaimStatus::PendingAttestation;
        },
    }
    claim.status_changed_at = clock.unix_timestamp;
    
    emit!(ClaimStatusChanged {
        claim_id: params.claim_id,
        old_status,
        new_status: claim.status,
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}

/// Determine AI decision based on scores
fn determine_ai_decision(
    confidence_bps: u16,
    fraud_score_bps: u16,
    min_auto_approve_confidence: u16,
    max_fraud_for_approval: u16,
    min_confidence_threshold: u16,
    flags: &[String],
) -> AiDecisionType {
    // High fraud score = auto-deny
    if fraud_score_bps > 9000 {
        return AiDecisionType::AutoDeny {
            reason: "High fraud probability detected".to_string(),
        };
    }
    
    // High confidence + low fraud = auto-approve
    if confidence_bps >= min_auto_approve_confidence 
        && fraud_score_bps <= max_fraud_for_approval
        && flags.is_empty()
    {
        return AiDecisionType::AutoApprove;
    }
    
    // Low confidence = committee review
    if confidence_bps < min_confidence_threshold {
        return AiDecisionType::CommitteeReview;
    }
    
    // Medium confidence or flags present = committee review
    AiDecisionType::CommitteeReview
}

/// Process fast-lane claim (immediate auto-approval for small routine claims)
#[derive(Accounts)]
#[instruction(claim_id: u64, month_start: i64)]
pub struct ProcessFastLane<'info> {
    #[account(
        mut,
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump = claims_config.bump,
        constraint = claims_config.is_active @ ClaimsError::ClaimsPaused
    )]
    pub claims_config: Account<'info, ClaimsConfig>,
    
    #[account(
        mut,
        seeds = [ClaimAccount::SEED_PREFIX, &claim_id.to_le_bytes()],
        bump = claim.bump,
        constraint = claim.status == ClaimStatus::Submitted @ ClaimsError::InvalidClaimStatus
    )]
    pub claim: Account<'info, ClaimAccount>,
    
    #[account(
        init_if_needed,
        payer = processor,
        space = 8 + FastLaneUsage::INIT_SPACE,
        seeds = [
            FastLaneUsage::SEED_PREFIX,
            claim.member.as_ref(),
            &month_start.to_le_bytes()
        ],
        bump
    )]
    pub fast_lane_usage: Account<'info, FastLaneUsage>,
    
    #[account(mut)]
    pub processor: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn process_fast_lane(ctx: Context<ProcessFastLane>, claim_id: u64, _month_start: i64) -> Result<()> {
    let clock = Clock::get()?;
    let config = &mut ctx.accounts.claims_config;
    let claim = &mut ctx.accounts.claim;
    let usage = &mut ctx.accounts.fast_lane_usage;
    
    // Check fast-lane eligibility
    require!(
        claim.requested_amount <= config.auto_approve_threshold,
        ClaimsError::ExceedsFastLaneLimit
    );
    
    // Check eligible categories (basic check - can expand)
    require!(
        is_fast_lane_category(&claim.category),
        ClaimsError::CategoryNotEligible
    );
    
    // Check monthly usage limit
    let month_start = get_month_start(clock.unix_timestamp);
    if usage.month_start != month_start {
        // New month, reset usage
        usage.member = claim.member;
        usage.month_start = month_start;
        usage.claims_used = 0;
        usage.amount_claimed = 0;
    }
    
    require!(
        usage.claims_used < 5, // Max 5 fast-lane per month
        ClaimsError::FastLaneLimitExceeded
    );
    
    // Approve via fast-lane
    let old_status = claim.status;
    claim.status = ClaimStatus::Approved;
    claim.approved_amount = claim.requested_amount;
    claim.status_changed_at = clock.unix_timestamp;
    
    // Update usage tracking
    usage.claims_used += 1;
    usage.amount_claimed = usage.amount_claimed.saturating_add(claim.requested_amount);
    
    // Update config stats
    config.total_claims_approved += 1;
    
    emit!(ClaimStatusChanged {
        claim_id,
        old_status,
        new_status: ClaimStatus::Approved,
        timestamp: clock.unix_timestamp,
    });
    
    // Emit fast-lane specific event
    emit!(FastLaneApproved {
        claim_id,
        member: claim.member,
        amount: claim.requested_amount,
        category: claim.category,
        monthly_usage: usage.claims_used,
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}

/// Check if category is eligible for fast-lane
fn is_fast_lane_category(category: &ClaimCategory) -> bool {
    matches!(
        category,
        ClaimCategory::PrimaryCare |
        ClaimCategory::Prescription |
        ClaimCategory::Laboratory |
        ClaimCategory::Preventive |
        ClaimCategory::DiagnosticImaging
    )
}

/// Get the start of the current month (Unix timestamp)
fn get_month_start(timestamp: i64) -> i64 {
    // Approximate: 30 days per month
    let days_since_epoch = timestamp / 86400;
    let months_since_epoch = days_since_epoch / 30;
    months_since_epoch * 30 * 86400
}

/// Mark an AI decision as overturned (for accuracy tracking)
#[derive(Accounts)]
#[instruction(claim_id: u64)]
pub struct MarkDecisionOverturned<'info> {
    #[account(
        mut,
        seeds = [AiOracle::SEED_PREFIX],
        bump = ai_oracle.bump,
    )]
    pub ai_oracle: Account<'info, AiOracle>,
    
    #[account(
        mut,
        seeds = [AiDecision::SEED_PREFIX, &claim_id.to_le_bytes()],
        bump = ai_decision.bump,
        constraint = !ai_decision.overturned @ ClaimsError::AlreadyOverturned
    )]
    pub ai_decision: Account<'info, AiDecision>,
    
    /// Must be committee or DAO
    #[account(
        constraint = authority.key() == ai_oracle.authority @ ClaimsError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn mark_decision_overturned(ctx: Context<MarkDecisionOverturned>, _claim_id: u64) -> Result<()> {
    let oracle = &mut ctx.accounts.ai_oracle;
    let decision = &mut ctx.accounts.ai_decision;
    
    decision.overturned = true;
    oracle.decisions_overturned += 1;
    
    // Update accuracy rate
    if oracle.total_decisions > 0 {
        let accurate = oracle.total_decisions - oracle.decisions_overturned;
        oracle.accuracy_rate_bps = ((accurate as u128 * 10000) / oracle.total_decisions as u128) as u16;
    }
    
    Ok(())
}

// =============================================================================
// EVENTS
// =============================================================================

#[event]
pub struct FastLaneApproved {
    pub claim_id: u64,
    pub member: Pubkey,
    pub amount: u64,
    pub category: ClaimCategory,
    pub monthly_usage: u8,
    pub timestamp: i64,
}

// AiDecisionRecorded is defined in events.rs to avoid duplicate discriminators
