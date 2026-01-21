use anchor_lang::prelude::*;

use crate::state::ReinsuranceConfig;
use crate::errors::ReinsuranceError;
use crate::events::ReinsuranceConfigInitialized;

/// Initialize the global reinsurance configuration
#[derive(Accounts)]
pub struct InitializeReinsurance<'info> {
    #[account(
        init,
        payer = authority,
        space = ReinsuranceConfig::SIZE,
        seeds = [b"reinsurance_config"],
        bump
    )]
    pub config: Account<'info, ReinsuranceConfig>,
    
    /// Authority (DAO multisig)
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// Reinsurance committee multisig
    /// CHECK: Validated as pubkey only
    pub reinsurance_committee: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

/// Parameters for initializing reinsurance
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeReinsuranceParams {
    /// Policy year start timestamp
    pub policy_year_start: i64,
    
    /// Policy year end timestamp
    pub policy_year_end: i64,
    
    /// Expected annual claims (USDC, 6 decimals)
    pub expected_annual_claims: u64,
    
    /// Annual budget for reinsurance premium (USDC, 6 decimals)
    pub premium_budget: u64,
    
    /// Aggregate trigger ratio (basis points, e.g., 11000 = 110%)
    pub aggregate_trigger_ratio_bps: u16,
    
    /// Catastrophic trigger ratio (basis points, e.g., 15000 = 150%)
    pub catastrophic_trigger_ratio_bps: u16,
    
    /// Catastrophic ceiling ratio (basis points, e.g., 30000 = 300%)
    pub catastrophic_ceiling_ratio_bps: u16,
}

pub fn handler(
    ctx: Context<InitializeReinsurance>,
    params: InitializeReinsuranceParams,
) -> Result<()> {
    // Validate dates
    require!(
        params.policy_year_start < params.policy_year_end,
        ReinsuranceError::InvalidPolicyYearDates
    );
    
    // Validate trigger ratios
    require!(
        params.aggregate_trigger_ratio_bps > 10000, // Must be > 100%
        ReinsuranceError::InvalidAggregateTrigger
    );
    
    require!(
        params.catastrophic_trigger_ratio_bps > params.aggregate_trigger_ratio_bps,
        ReinsuranceError::InvalidCatastrophicTrigger
    );
    
    require!(
        params.catastrophic_ceiling_ratio_bps > params.catastrophic_trigger_ratio_bps,
        ReinsuranceError::InvalidCeilingRatio
    );
    
    require!(
        params.expected_annual_claims > 0,
        ReinsuranceError::ExpectedClaimsNotSet
    );
    
    let config = &mut ctx.accounts.config;
    let clock = Clock::get()?;
    
    config.authority = ctx.accounts.authority.key();
    config.reinsurance_committee = ctx.accounts.reinsurance_committee.key();
    config.policy_year_start = params.policy_year_start;
    config.policy_year_end = params.policy_year_end;
    config.expected_annual_claims = params.expected_annual_claims;
    config.premium_budget = params.premium_budget;
    config.aggregate_trigger_ratio_bps = params.aggregate_trigger_ratio_bps;
    config.catastrophic_trigger_ratio_bps = params.catastrophic_trigger_ratio_bps;
    config.catastrophic_ceiling_ratio_bps = params.catastrophic_ceiling_ratio_bps;
    config.bump = ctx.bumps.config;
    
    // Initialize counters to zero (default)
    config.ytd_claims_paid = 0;
    config.ytd_recoveries_received = 0;
    config.total_treaties = 0;
    config.active_treaties = 0;
    config.total_recovery_claims = 0;
    config.pending_recoveries = 0;
    config.premium_paid_ytd = 0;
    config.aggregate_triggered = false;
    config.catastrophic_triggered = false;
    
    emit!(ReinsuranceConfigInitialized {
        authority: config.authority,
        reinsurance_committee: config.reinsurance_committee,
        policy_year_start: config.policy_year_start,
        policy_year_end: config.policy_year_end,
        expected_annual_claims: config.expected_annual_claims,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Reinsurance config initialized");
    msg!("Expected annual claims: {} USDC", params.expected_annual_claims / 1_000_000);
    msg!("Aggregate trigger: {}%", params.aggregate_trigger_ratio_bps / 100);
    msg!("Catastrophic trigger: {}%", params.catastrophic_trigger_ratio_bps / 100);
    
    Ok(())
}

// ============================================================================
// UPDATE CONFIGURATION
// ============================================================================

#[derive(Accounts)]
pub struct UpdateReinsuranceConfig<'info> {
    #[account(
        mut,
        seeds = [b"reinsurance_config"],
        bump = config.bump,
        has_one = authority @ ReinsuranceError::Unauthorized,
    )]
    pub config: Account<'info, ReinsuranceConfig>,
    
    pub authority: Signer<'info>,
}

/// Update expected annual claims (e.g., mid-year adjustment)
pub fn update_expected_claims(
    ctx: Context<UpdateReinsuranceConfig>,
    new_expected_claims: u64,
    reason_hash: [u8; 32],
) -> Result<()> {
    require!(new_expected_claims > 0, ReinsuranceError::ZeroAmount);
    
    let config = &mut ctx.accounts.config;
    let clock = Clock::get()?;
    
    let old_expected = config.expected_annual_claims;
    config.expected_annual_claims = new_expected_claims;
    
    emit!(crate::events::ExpectedClaimsUpdated {
        old_expected,
        new_expected: new_expected_claims,
        updater: ctx.accounts.authority.key(),
        reason: reason_hash,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Expected claims updated: {} -> {} USDC", 
        old_expected / 1_000_000, 
        new_expected_claims / 1_000_000
    );
    
    Ok(())
}

/// Update trigger ratios
pub fn update_trigger_ratios(
    ctx: Context<UpdateReinsuranceConfig>,
    aggregate_trigger_bps: u16,
    catastrophic_trigger_bps: u16,
    catastrophic_ceiling_bps: u16,
) -> Result<()> {
    require!(
        aggregate_trigger_bps > 10000,
        ReinsuranceError::InvalidAggregateTrigger
    );
    
    require!(
        catastrophic_trigger_bps > aggregate_trigger_bps,
        ReinsuranceError::InvalidCatastrophicTrigger
    );
    
    require!(
        catastrophic_ceiling_bps > catastrophic_trigger_bps,
        ReinsuranceError::InvalidCeilingRatio
    );
    
    let config = &mut ctx.accounts.config;
    let clock = Clock::get()?;
    
    let old_agg = config.aggregate_trigger_ratio_bps;
    let old_cat = config.catastrophic_trigger_ratio_bps;
    let old_ceil = config.catastrophic_ceiling_ratio_bps;
    
    config.aggregate_trigger_ratio_bps = aggregate_trigger_bps;
    config.catastrophic_trigger_ratio_bps = catastrophic_trigger_bps;
    config.catastrophic_ceiling_ratio_bps = catastrophic_ceiling_bps;
    
    emit!(crate::events::TriggerRatiosUpdated {
        old_aggregate_bps: old_agg,
        new_aggregate_bps: aggregate_trigger_bps,
        old_catastrophic_bps: old_cat,
        new_catastrophic_bps: catastrophic_trigger_bps,
        old_ceiling_bps: old_ceil,
        new_ceiling_bps: catastrophic_ceiling_bps,
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}

/// Start new policy year
pub fn start_new_policy_year(
    ctx: Context<UpdateReinsuranceConfig>,
    new_year_start: i64,
    new_year_end: i64,
    new_expected_claims: u64,
    new_premium_budget: u64,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let clock = Clock::get()?;
    
    // Ensure old year has ended
    require!(
        clock.unix_timestamp > config.policy_year_end,
        ReinsuranceError::CannotResetDuringActiveYear
    );
    
    require!(
        new_year_start < new_year_end,
        ReinsuranceError::InvalidPolicyYearDates
    );
    
    let old_start = config.policy_year_start;
    let old_end = config.policy_year_end;
    
    // Emit year-end reconciliation before reset
    emit!(crate::events::YearEndReconciliation {
        policy_year_start: old_start,
        policy_year_end: old_end,
        total_claims_paid: config.ytd_claims_paid,
        expected_claims: config.expected_annual_claims,
        total_recoveries_filed: config.total_recovery_claims,
        total_recoveries_received: config.ytd_recoveries_received,
        total_premium_paid: config.premium_paid_ytd,
        aggregate_triggered: config.aggregate_triggered,
        catastrophic_triggered: config.catastrophic_triggered,
        net_reinsurance_benefit: (config.ytd_recoveries_received as i64)
            .saturating_sub(config.premium_paid_ytd as i64),
        timestamp: clock.unix_timestamp,
    });
    
    // Reset for new year
    config.policy_year_start = new_year_start;
    config.policy_year_end = new_year_end;
    config.expected_annual_claims = new_expected_claims;
    config.premium_budget = new_premium_budget;
    config.ytd_claims_paid = 0;
    config.ytd_recoveries_received = 0;
    config.premium_paid_ytd = 0;
    config.pending_recoveries = 0;
    config.aggregate_triggered = false;
    config.catastrophic_triggered = false;
    // Note: total_recovery_claims is cumulative, not reset
    // Note: active_treaties may carry over
    
    emit!(crate::events::PolicyYearUpdated {
        old_year_start: old_start,
        old_year_end: old_end,
        new_year_start,
        new_year_end,
        expected_annual_claims: new_expected_claims,
        ytd_claims_reset: true,
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}
