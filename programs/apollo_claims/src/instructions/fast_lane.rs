// programs/apollo_claims/src/instructions/fast_lane.rs
//
// Fast-Lane Auto-Approval Instructions
// Tier 1 of three-tier claims processing
// Instantly approves small routine claims without human review

use super::ai_processing::FastLaneApproved;
use crate::ai_oracle::{FastLaneConfig, FastLaneTracker};
use crate::errors::ClaimsError;
use crate::state::{ClaimAccount, ClaimCategory, ClaimStatus, ClaimsConfig};
use anchor_lang::prelude::*;

// =============================================================================
// INITIALIZATION
// =============================================================================

#[derive(Accounts)]
pub struct InitializeFastLane<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + FastLaneConfig::INIT_SPACE,
        seeds = [FastLaneConfig::SEED_PREFIX],
        bump
    )]
    pub fast_lane_config: Account<'info, FastLaneConfig>,

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
pub struct InitializeFastLaneParams {
    /// Maximum amount for fast-lane (USDC lamports)
    pub max_amount: u64,
    /// Eligible category codes (ClaimCategory as u8)
    pub eligible_categories: Vec<u8>,
    /// Max claims per member per period
    pub max_claims_per_period: u8,
    /// Minimum member tenure in days
    pub min_member_tenure_days: u16,
    /// Is bootstrap mode (more conservative)
    pub bootstrap_mode: bool,
}

pub fn initialize_fast_lane(
    ctx: Context<InitializeFastLane>,
    params: InitializeFastLaneParams,
) -> Result<()> {
    let config = &mut ctx.accounts.fast_lane_config;

    // Use bootstrap or standard defaults
    let max_amount = if params.bootstrap_mode {
        params.max_amount.min(FastLaneConfig::BOOTSTRAP_MAX_AMOUNT)
    } else {
        params.max_amount.min(FastLaneConfig::STANDARD_MAX_AMOUNT)
    };

    let max_claims = if params.bootstrap_mode {
        params
            .max_claims_per_period
            .min(FastLaneConfig::BOOTSTRAP_MAX_CLAIMS)
    } else {
        params
            .max_claims_per_period
            .min(FastLaneConfig::STANDARD_MAX_CLAIMS)
    };

    config.authority = ctx.accounts.authority.key();
    config.max_amount = max_amount;
    config.eligible_categories = params.eligible_categories;
    config.max_claims_per_period = max_claims;
    config.period_seconds = FastLaneConfig::DEFAULT_PERIOD_SECONDS;
    config.min_member_tenure_days = params.min_member_tenure_days;
    config.is_active = true;
    config.total_fast_lane_approvals = 0;
    config.total_fast_lane_paid = 0;
    config.bump = ctx.bumps.fast_lane_config;

    Ok(())
}

// =============================================================================
// UPDATE CONFIGURATION
// =============================================================================

#[derive(Accounts)]
pub struct UpdateFastLaneConfig<'info> {
    #[account(
        mut,
        seeds = [FastLaneConfig::SEED_PREFIX],
        bump = fast_lane_config.bump,
        constraint = fast_lane_config.authority == authority.key() @ ClaimsError::Unauthorized
    )]
    pub fast_lane_config: Account<'info, FastLaneConfig>,

    pub authority: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateFastLaneParams {
    pub max_amount: Option<u64>,
    pub eligible_categories: Option<Vec<u8>>,
    pub max_claims_per_period: Option<u8>,
    pub min_member_tenure_days: Option<u16>,
    pub is_active: Option<bool>,
}

pub fn update_fast_lane_config(
    ctx: Context<UpdateFastLaneConfig>,
    params: UpdateFastLaneParams,
) -> Result<()> {
    let config = &mut ctx.accounts.fast_lane_config;

    if let Some(max_amount) = params.max_amount {
        config.max_amount = max_amount;
    }
    if let Some(categories) = params.eligible_categories {
        config.eligible_categories = categories;
    }
    if let Some(max_claims) = params.max_claims_per_period {
        config.max_claims_per_period = max_claims;
    }
    if let Some(tenure) = params.min_member_tenure_days {
        config.min_member_tenure_days = tenure;
    }
    if let Some(active) = params.is_active {
        config.is_active = active;
    }

    Ok(())
}

// =============================================================================
// FAST-LANE PROCESSING
// =============================================================================

#[derive(Accounts)]
pub struct ProcessFastLane<'info> {
    #[account(
        seeds = [FastLaneConfig::SEED_PREFIX],
        bump = fast_lane_config.bump,
        constraint = fast_lane_config.is_active @ ClaimsError::FastLaneDisabled
    )]
    pub fast_lane_config: Account<'info, FastLaneConfig>,

    #[account(
        mut,
        seeds = [ClaimAccount::SEED_PREFIX, &claim.claim_id.to_le_bytes()],
        bump = claim.bump,
        constraint = claim.status == ClaimStatus::Submitted @ ClaimsError::InvalidClaimStatus
    )]
    pub claim: Account<'info, ClaimAccount>,

    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + FastLaneTracker::INIT_SPACE,
        seeds = [FastLaneTracker::SEED_PREFIX, claim.member.as_ref()],
        bump
    )]
    pub tracker: Account<'info, FastLaneTracker>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Process a claim through fast-lane
/// Returns Ok(true) if auto-approved, Ok(false) if routed to next tier
pub fn process_fast_lane(ctx: Context<ProcessFastLane>) -> Result<bool> {
    let clock = Clock::get()?;
    let config = &ctx.accounts.fast_lane_config;
    let claim = &mut ctx.accounts.claim;
    let tracker = &mut ctx.accounts.tracker;

    // Initialize tracker if new
    if tracker.member == Pubkey::default() {
        tracker.member = claim.member;
        tracker.period_start = clock.unix_timestamp;
        tracker.bump = ctx.bumps.tracker;
    }

    // Check eligibility
    let eligible = check_fast_lane_eligibility(config, claim, tracker, clock.unix_timestamp)?;

    if !eligible {
        // Route to AI triage (Tier 2)
        claim.status = ClaimStatus::UnderReview;
        claim.status_changed_at = clock.unix_timestamp;
        return Ok(false);
    }

    // Auto-approve
    claim.status = ClaimStatus::Approved;
    claim.approved_amount = claim.requested_amount;
    claim.status_changed_at = clock.unix_timestamp;

    // Update tracker
    tracker.record_claim(claim.requested_amount, config, clock.unix_timestamp);

    // Update config stats (would need mut, simplified here)
    // In production: config.total_fast_lane_approvals += 1;
    // config.total_fast_lane_paid += claim.requested_amount;

    emit!(FastLaneApproved {
        claim_id: claim.claim_id,
        member: claim.member,
        amount: claim.requested_amount,
        category: claim.category,
        monthly_usage: 0, // tracked separately in FastLaneTracker
        timestamp: clock.unix_timestamp,
    });

    Ok(true)
}

/// Check if a claim is eligible for fast-lane processing
fn check_fast_lane_eligibility(
    config: &FastLaneConfig,
    claim: &ClaimAccount,
    tracker: &FastLaneTracker,
    current_time: i64,
) -> Result<bool> {
    // Check amount threshold
    if claim.requested_amount > config.max_amount {
        return Ok(false);
    }

    // Check category eligibility
    let category_code = claim.category as u8;
    if !config.eligible_categories.contains(&category_code) {
        return Ok(false);
    }

    // Check member's fast-lane usage
    if !tracker.can_use_fast_lane(config, current_time) {
        return Ok(false);
    }

    // Check if flagged
    if tracker.flagged {
        return Ok(false);
    }

    // Check if shock claim
    if claim.is_shock_claim {
        return Ok(false);
    }

    Ok(true)
}

/// Default eligible categories for fast-lane
/// These are low-risk, routine claim types
pub fn default_fast_lane_categories() -> Vec<u8> {
    vec![
        ClaimCategory::PrimaryCare as u8,
        ClaimCategory::Preventive as u8,
        ClaimCategory::Laboratory as u8,
        ClaimCategory::Prescription as u8,
        ClaimCategory::SpecialistVisit as u8,
    ]
}
