use anchor_lang::prelude::*;

use crate::state::{ReinsuranceConfig, ReinsuranceTreaty, MemberClaimsAccumulator, MonthlyAggregate, ReinsuranceLayerType};
use crate::errors::ReinsuranceError;
use crate::events::*;

// ============================================================================
// CREATE MEMBER ACCUMULATOR
// ============================================================================

#[derive(Accounts)]
#[instruction(member: Pubkey, policy_year: u16)]
pub struct CreateMemberAccumulator<'info> {
    #[account(
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,
    
    #[account(
        init,
        payer = payer,
        space = MemberClaimsAccumulator::SIZE,
        seeds = [
            b"member_accumulator",
            member.as_ref(),
            &policy_year.to_le_bytes()
        ],
        bump
    )]
    pub accumulator: Account<'info, MemberClaimsAccumulator>,
    
    #[account(mut)]
    pub payer: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn create_member_accumulator(
    ctx: Context<CreateMemberAccumulator>,
    member: Pubkey,
    policy_year: u16,
) -> Result<()> {
    let accumulator = &mut ctx.accounts.accumulator;
    let clock = Clock::get()?;
    
    accumulator.member = member;
    accumulator.policy_year = policy_year;
    accumulator.ytd_claims = 0;
    accumulator.claims_count = 0;
    accumulator.excess_claimed = 0;
    accumulator.recovered_amount = 0;
    accumulator.max_single_claim = 0;
    accumulator.stop_loss_triggered = false;
    accumulator.first_trigger_timestamp = 0;
    accumulator.last_claim_timestamp = 0;
    accumulator.bump = ctx.bumps.accumulator;
    
    emit!(MemberAccumulatorCreated {
        member,
        policy_year,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Member accumulator created for year {}", policy_year);
    
    Ok(())
}

// ============================================================================
// RECORD CLAIM TO ACCUMULATOR
// ============================================================================

#[derive(Accounts)]
pub struct RecordClaimToAccumulator<'info> {
    #[account(
        mut,
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,
    
    /// Specific stop-loss treaty to check against
    #[account(
        constraint = treaty.layer_type == ReinsuranceLayerType::SpecificStopLoss 
            @ ReinsuranceError::TreatyTypeMismatch,
    )]
    pub treaty: Account<'info, ReinsuranceTreaty>,
    
    #[account(mut)]
    pub accumulator: Account<'info, MemberClaimsAccumulator>,
    
    /// Authority must be claims program or authorized caller
    pub authority: Signer<'info>,
}

pub fn record_claim_to_accumulator(
    ctx: Context<RecordClaimToAccumulator>,
    claim_amount: u64,
    original_claim_id: u64,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let treaty = &ctx.accounts.treaty;
    let accumulator = &mut ctx.accounts.accumulator;
    let clock = Clock::get()?;
    
    require!(claim_amount > 0, ReinsuranceError::ZeroAmount);
    
    // Check if this triggers stop-loss
    let was_triggered = accumulator.stop_loss_triggered;
    let excess = accumulator.check_stop_loss_trigger(claim_amount, treaty.attachment_point);
    
    // Update accumulator
    accumulator.ytd_claims = accumulator.ytd_claims.checked_add(claim_amount)
        .ok_or(ReinsuranceError::Overflow)?;
    accumulator.claims_count = accumulator.claims_count.checked_add(1)
        .ok_or(ReinsuranceError::Overflow)?;
    accumulator.last_claim_timestamp = clock.unix_timestamp;
    
    if claim_amount > accumulator.max_single_claim {
        accumulator.max_single_claim = claim_amount;
    }
    
    // Update global YTD claims
    config.ytd_claims_paid = config.ytd_claims_paid.checked_add(claim_amount)
        .ok_or(ReinsuranceError::Overflow)?;
    
    emit!(MemberClaimRecorded {
        member: accumulator.member,
        claim_amount,
        ytd_total: accumulator.ytd_claims,
        claims_count: accumulator.claims_count,
        original_claim_id,
        timestamp: clock.unix_timestamp,
    });
    
    // If just triggered stop-loss
    if let Some(excess_amount) = excess {
        if !was_triggered {
            accumulator.stop_loss_triggered = true;
            accumulator.first_trigger_timestamp = clock.unix_timestamp;
            accumulator.excess_claimed = excess_amount;
            
            emit!(MemberStopLossBreached {
                member: accumulator.member,
                ytd_claims: accumulator.ytd_claims,
                attachment_point: treaty.attachment_point,
                excess_amount,
                breach_timestamp: clock.unix_timestamp,
            });
            
            msg!("Stop-loss triggered for member: {} USDC excess", 
                excess_amount / 1_000_000
            );
        } else {
            // Already triggered, update excess
            accumulator.excess_claimed = accumulator.ytd_claims
                .saturating_sub(treaty.attachment_point);
        }
    }
    
    Ok(())
}

// ============================================================================
// UPDATE RECOVERY AMOUNT
// ============================================================================

#[derive(Accounts)]
pub struct UpdateAccumulatorRecovery<'info> {
    #[account(
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,
    
    #[account(mut)]
    pub accumulator: Account<'info, MemberClaimsAccumulator>,
    
    pub authority: Signer<'info>,
}

pub fn update_accumulator_recovery(
    ctx: Context<UpdateAccumulatorRecovery>,
    recovered_amount: u64,
) -> Result<()> {
    let config = &ctx.accounts.config;
    let accumulator = &mut ctx.accounts.accumulator;
    
    // Validate authority
    let is_authorized = ctx.accounts.authority.key() == config.authority
        || ctx.accounts.authority.key() == config.reinsurance_committee;
    require!(is_authorized, ReinsuranceError::Unauthorized);
    
    accumulator.recovered_amount = accumulator.recovered_amount
        .checked_add(recovered_amount)
        .ok_or(ReinsuranceError::Overflow)?;
    
    msg!("Accumulator recovery updated: {} USDC total recovered",
        accumulator.recovered_amount / 1_000_000
    );
    
    Ok(())
}

// ============================================================================
// MONTHLY AGGREGATE TRACKING
// ============================================================================

#[derive(Accounts)]
#[instruction(policy_year: u16, month: u8)]
pub struct InitializeMonthlyAggregate<'info> {
    #[account(
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,
    
    #[account(
        init,
        payer = payer,
        space = MonthlyAggregate::SIZE,
        seeds = [
            b"monthly_aggregate",
            policy_year.to_le_bytes().as_ref(),
            &[month]
        ],
        bump
    )]
    pub monthly: Account<'info, MonthlyAggregate>,
    
    #[account(mut)]
    pub payer: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn initialize_monthly_aggregate(
    ctx: Context<InitializeMonthlyAggregate>,
    policy_year: u16,
    month: u8,
    expected_claims: u64,
) -> Result<()> {
    let monthly = &mut ctx.accounts.monthly;
    
    require!(month >= 1 && month <= 12, ReinsuranceError::InvalidConfiguration);
    
    monthly.policy_year = policy_year;
    monthly.month = month;
    monthly.expected_claims = expected_claims;
    monthly.bump = ctx.bumps.monthly;
    
    // Initialize counters
    monthly.total_claims = 0;
    monthly.claims_count = 0;
    monthly.avg_claim_amount = 0;
    monthly.ratio_bps = 0;
    monthly.ytd_through_month = 0;
    monthly.max_claim = 0;
    monthly.shock_claims_count = 0;
    monthly.last_updated = 0;
    
    msg!("Monthly aggregate initialized for {}/{}", policy_year, month);
    
    Ok(())
}

#[derive(Accounts)]
pub struct UpdateMonthlyAggregate<'info> {
    #[account(
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,
    
    #[account(mut)]
    pub monthly: Account<'info, MonthlyAggregate>,
    
    pub authority: Signer<'info>,
}

pub fn update_monthly_aggregate(
    ctx: Context<UpdateMonthlyAggregate>,
    claim_amount: u64,
    is_shock_claim: bool,
    ytd_total: u64,
) -> Result<()> {
    let monthly = &mut ctx.accounts.monthly;
    let clock = Clock::get()?;
    
    monthly.total_claims = monthly.total_claims.checked_add(claim_amount)
        .ok_or(ReinsuranceError::Overflow)?;
    monthly.claims_count = monthly.claims_count.checked_add(1)
        .ok_or(ReinsuranceError::Overflow)?;
    
    if monthly.claims_count > 0 {
        monthly.avg_claim_amount = monthly.total_claims / monthly.claims_count as u64;
    }
    
    if claim_amount > monthly.max_claim {
        monthly.max_claim = claim_amount;
    }
    
    if is_shock_claim {
        monthly.shock_claims_count = monthly.shock_claims_count.checked_add(1)
            .ok_or(ReinsuranceError::Overflow)?;
    }
    
    monthly.ytd_through_month = ytd_total;
    
    // Calculate ratio
    if monthly.expected_claims > 0 {
        monthly.ratio_bps = ((monthly.total_claims as u128 * 10_000) / monthly.expected_claims as u128) as u16;
    }
    
    monthly.last_updated = clock.unix_timestamp;
    
    emit!(MonthlyAggregateUpdated {
        policy_year: monthly.policy_year,
        month: monthly.month,
        total_claims: monthly.total_claims,
        claims_count: monthly.claims_count,
        expected_claims: monthly.expected_claims,
        ratio_bps: monthly.ratio_bps,
        ytd_through_month: monthly.ytd_through_month,
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}

// ============================================================================
// RESET ACCUMULATORS (YEAR END)
// ============================================================================

#[derive(Accounts)]
pub struct ResetAccumulators<'info> {
    #[account(
        mut,
        seeds = [b"reinsurance_config"],
        bump = config.bump,
        has_one = authority @ ReinsuranceError::Unauthorized,
    )]
    pub config: Account<'info, ReinsuranceConfig>,
    
    pub authority: Signer<'info>,
}

/// Called at year end to note that accumulators should be reset
/// Individual accumulator accounts will be created fresh for new year
pub fn mark_accumulators_for_reset(ctx: Context<ResetAccumulators>) -> Result<()> {
    let config = &ctx.accounts.config;
    let clock = Clock::get()?;
    
    // Ensure policy year has ended
    require!(
        clock.unix_timestamp > config.policy_year_end,
        ReinsuranceError::CannotResetDuringActiveYear
    );
    
    msg!("Accumulators marked for reset. Create new accumulators for next policy year.");
    
    emit!(AccumulatorsReset {
        policy_year: 0, // Will be filled by caller
        members_reset: 0,
        total_ytd_reset: config.ytd_claims_paid,
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}

// ============================================================================
// CHECK AGGREGATE THRESHOLDS
// ============================================================================

#[derive(Accounts)]
pub struct CheckAggregateThresholds<'info> {
    #[account(
        mut,
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,
}

/// Permissionless check if aggregate thresholds have been crossed
pub fn check_aggregate_thresholds(ctx: Context<CheckAggregateThresholds>) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let clock = Clock::get()?;
    
    let current_ratio = config.current_claims_ratio_bps();
    
    // Check warning threshold (e.g., 100% of expected)
    if current_ratio >= 10_000 && !config.aggregate_triggered {
        let headroom = if config.aggregate_trigger_ratio_bps as u64 > current_ratio {
            let trigger_amount = config.expected_annual_claims
                .checked_mul(config.aggregate_trigger_ratio_bps as u64)
                .unwrap_or(0)
                .checked_div(10_000)
                .unwrap_or(0);
            trigger_amount.saturating_sub(config.ytd_claims_paid)
        } else {
            0
        };
        
        emit!(ClaimsThresholdWarning {
            current_ytd: config.ytd_claims_paid,
            expected_annual: config.expected_annual_claims,
            current_ratio_bps: current_ratio,
            warning_threshold_bps: 10_000,
            aggregate_trigger_bps: config.aggregate_trigger_ratio_bps,
            remaining_headroom: headroom,
            timestamp: clock.unix_timestamp,
        });
        
        msg!("Warning: Claims at {}% of expected", current_ratio / 100);
    }
    
    // Check if aggregate should trigger
    if config.should_trigger_aggregate() {
        config.aggregate_triggered = true;
        
        emit!(AggregateStopLossTriggered {
            treaty_id: 0, // Will be filled when filing recovery
            ytd_claims: config.ytd_claims_paid,
            expected_claims: config.expected_annual_claims,
            trigger_ratio_bps: config.aggregate_trigger_ratio_bps,
            actual_ratio_bps: current_ratio,
            excess_amount: config.calculate_aggregate_recoverable(),
            timestamp: clock.unix_timestamp,
        });
        
        msg!("AGGREGATE STOP-LOSS TRIGGERED at {}% of expected", current_ratio / 100);
    }
    
    // Check if catastrophic should trigger
    if config.should_trigger_catastrophic() {
        config.catastrophic_triggered = true;
        
        emit!(CatastrophicLayerTriggered {
            treaty_id: 0,
            ytd_claims: config.ytd_claims_paid,
            expected_claims: config.expected_annual_claims,
            trigger_ratio_bps: config.catastrophic_trigger_ratio_bps,
            actual_ratio_bps: current_ratio,
            timestamp: clock.unix_timestamp,
        });
        
        msg!("CATASTROPHIC LAYER TRIGGERED at {}% of expected", current_ratio / 100);
    }
    
    Ok(())
}
