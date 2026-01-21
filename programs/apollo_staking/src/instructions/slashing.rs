// programs/apollo_staking/src/instructions/slashing.rs

use anchor_lang::prelude::*;
use crate::state::{StakingConfig, StakingTier, StakePosition, AphVault, LiquidationQueue, LiquidationEntry};
use crate::errors::StakingError;
use crate::events::{Slashed, PositionSlashed, LiquidationQueued, LiquidationExecuted, CircuitBreakerTriggered};

/// Slash a specific position (called during claim shortfall)
#[derive(Accounts)]
pub struct SlashPosition<'info> {
    #[account(
        seeds = [StakingConfig::SEED_PREFIX],
        bump = staking_config.bump,
    )]
    pub staking_config: Account<'info, StakingConfig>,

    #[account(
        mut,
        seeds = [StakingTier::SEED_PREFIX, &[stake_position.tier_id]],
        bump = staking_tier.bump,
    )]
    pub staking_tier: Account<'info, StakingTier>,

    #[account(
        mut,
        seeds = [
            StakePosition::SEED_PREFIX,
            stake_position.staker.as_ref(),
            &stake_position.position_id.to_le_bytes()
        ],
        bump = stake_position.bump,
        constraint = stake_position.is_active @ StakingError::PositionAlreadyClosed
    )]
    pub stake_position: Account<'info, StakePosition>,

    #[account(
        mut,
        seeds = [LiquidationQueue::SEED_PREFIX],
        bump = liquidation_queue.bump,
    )]
    pub liquidation_queue: Account<'info, LiquidationQueue>,

    /// Must be authorized (DAO or reserves program via CPI)
    #[account(
        constraint = authority.key() == staking_config.authority @ StakingError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn slash_position(
    ctx: Context<SlashPosition>,
    slash_amount: u64,
    target_usdc_value: u64,
    reason: String,
) -> Result<()> {
    let clock = Clock::get()?;
    let tier = &mut ctx.accounts.staking_tier;
    let position = &mut ctx.accounts.stake_position;
    let liq_queue = &mut ctx.accounts.liquidation_queue;

    let effective_stake = position.effective_stake();

    // Calculate max slashable based on tier's max loss
    let max_slash = effective_stake
        .saturating_mul(tier.max_loss_bps as u64)
        .checked_div(10000)
        .unwrap_or(0);

    let actual_slash = slash_amount.min(max_slash).min(effective_stake);
    require!(actual_slash > 0, StakingError::SlashExceedsPosition);

    // Apply slash to position
    position.slash_amount = position.slash_amount.saturating_add(actual_slash);
    position.was_slashed = true;

    // Update tier totals
    tier.total_staked = tier.total_staked.saturating_sub(actual_slash);

    // Queue slashed APH for liquidation
    require!(
        liq_queue.entries.len() < 50,
        StakingError::LiquidationQueueFull
    );

    let twap_hours = liq_queue.twap_window_hours as i64;
    let twap_end = clock.unix_timestamp + (twap_hours * 60 * 60);

    liq_queue.entries.push(LiquidationEntry {
        aph_amount: actual_slash,
        target_usdc: target_usdc_value,
        created_at: clock.unix_timestamp,
        start_at: clock.unix_timestamp,
        end_at: twap_end,
        liquidated_aph: 0,
        received_usdc: 0,
        is_complete: false,
    });

    liq_queue.pending_liquidation = liq_queue.pending_liquidation.saturating_add(actual_slash);

    emit!(PositionSlashed {
        staker: position.staker,
        position_id: position.position_id,
        slash_amount: actual_slash,
        remaining_amount: position.effective_stake(),
        timestamp: clock.unix_timestamp,
    });

    emit!(LiquidationQueued {
        aph_amount: actual_slash,
        target_usdc: target_usdc_value,
        twap_start: clock.unix_timestamp,
        twap_end,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Slash across an entire tier (proportional)
#[derive(Accounts)]
pub struct SlashTier<'info> {
    #[account(
        mut,
        seeds = [StakingConfig::SEED_PREFIX],
        bump = staking_config.bump,
    )]
    pub staking_config: Account<'info, StakingConfig>,

    #[account(
        mut,
        seeds = [StakingTier::SEED_PREFIX, &[staking_tier.tier_id]],
        bump = staking_tier.bump,
    )]
    pub staking_tier: Account<'info, StakingTier>,

    #[account(
        mut,
        seeds = [LiquidationQueue::SEED_PREFIX],
        bump = liquidation_queue.bump,
    )]
    pub liquidation_queue: Account<'info, LiquidationQueue>,

    #[account(
        constraint = authority.key() == staking_config.authority @ StakingError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn slash_tier(
    ctx: Context<SlashTier>,
    total_slash_amount: u64,
    target_usdc_value: u64,
    reason: String,
) -> Result<()> {
    let clock = Clock::get()?;
    let config = &mut ctx.accounts.staking_config;
    let tier = &mut ctx.accounts.staking_tier;
    let liq_queue = &mut ctx.accounts.liquidation_queue;

    // Calculate max slashable for the tier
    let max_tier_slash = tier.total_staked
        .saturating_mul(tier.max_loss_bps as u64)
        .checked_div(10000)
        .unwrap_or(0);

    let actual_slash = total_slash_amount.min(max_tier_slash).min(tier.total_staked);
    require!(actual_slash > 0, StakingError::SlashExceedsPosition);

    // Update tier totals (individual positions updated separately)
    tier.total_staked = tier.total_staked.saturating_sub(actual_slash);
    config.total_staked = config.total_staked.saturating_sub(actual_slash);

    // Queue for liquidation
    require!(
        liq_queue.entries.len() < 50,
        StakingError::LiquidationQueueFull
    );

    let twap_hours = liq_queue.twap_window_hours as i64;
    let twap_end = clock.unix_timestamp + (twap_hours * 60 * 60);

    liq_queue.entries.push(LiquidationEntry {
        aph_amount: actual_slash,
        target_usdc: target_usdc_value,
        created_at: clock.unix_timestamp,
        start_at: clock.unix_timestamp,
        end_at: twap_end,
        liquidated_aph: 0,
        received_usdc: 0,
        is_complete: false,
    });

    liq_queue.pending_liquidation = liq_queue.pending_liquidation.saturating_add(actual_slash);

    emit!(Slashed {
        tier_id: tier.tier_id,
        total_slashed: actual_slash,
        positions_affected: tier.staker_count as u32,
        reason,
        timestamp: clock.unix_timestamp,
    });

    emit!(LiquidationQueued {
        aph_amount: actual_slash,
        target_usdc: target_usdc_value,
        twap_start: clock.unix_timestamp,
        twap_end,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Execute liquidation (swap APH for USDC via DEX)
/// NOTE: In production, this would integrate with Jupiter or Raydium
#[derive(Accounts)]
pub struct ExecuteLiquidation<'info> {
    #[account(
        seeds = [StakingConfig::SEED_PREFIX],
        bump = staking_config.bump,
    )]
    pub staking_config: Account<'info, StakingConfig>,

    #[account(
        mut,
        seeds = [AphVault::SEED_PREFIX],
        bump = aph_vault.bump,
    )]
    pub aph_vault: Account<'info, AphVault>,

    #[account(
        mut,
        seeds = [LiquidationQueue::SEED_PREFIX],
        bump = liquidation_queue.bump,
        constraint = !liquidation_queue.is_paused @ StakingError::CircuitBreakerTriggered
    )]
    pub liquidation_queue: Account<'info, LiquidationQueue>,

    /// Liquidation executor (can be anyone for permissionless execution)
    pub executor: Signer<'info>,
}

pub fn execute_liquidation(
    ctx: Context<ExecuteLiquidation>,
    entry_index: u8,
    aph_sold: u64,
    usdc_received: u64,
) -> Result<()> {
    let clock = Clock::get()?;
    let liq_queue = &mut ctx.accounts.liquidation_queue;
    let vault = &mut ctx.accounts.aph_vault;

    require!(
        (entry_index as usize) < liq_queue.entries.len(),
        StakingError::PositionNotFound
    );

    let entry = &mut liq_queue.entries[entry_index as usize];

    require!(!entry.is_complete, StakingError::PositionAlreadyClosed);
    require!(
        clock.unix_timestamp <= entry.end_at,
        StakingError::TwapWindowNotComplete
    );

    // Check slippage
    let expected_usdc_per_aph = if entry.aph_amount > 0 {
        entry.target_usdc / entry.aph_amount
    } else {
        0
    };

    let actual_usdc_per_aph = if aph_sold > 0 {
        usdc_received / aph_sold
    } else {
        0
    };

    // Calculate slippage in basis points
    let slippage_bps = if expected_usdc_per_aph > 0 {
        let diff = if actual_usdc_per_aph > expected_usdc_per_aph {
            0
        } else {
            ((expected_usdc_per_aph - actual_usdc_per_aph) * 10000) / expected_usdc_per_aph
        };
        diff as u16
    } else {
        0
    };

    // Check circuit breaker
    if slippage_bps > liq_queue.circuit_breaker_bps {
        liq_queue.is_paused = true;

        emit!(CircuitBreakerTriggered {
            slippage_observed_bps: slippage_bps,
            threshold_bps: liq_queue.circuit_breaker_bps,
            timestamp: clock.unix_timestamp,
        });

        return Err(StakingError::SlippageExceeded.into());
    }

    // Update entry
    entry.liquidated_aph = entry.liquidated_aph.saturating_add(aph_sold);
    entry.received_usdc = entry.received_usdc.saturating_add(usdc_received);

    if entry.liquidated_aph >= entry.aph_amount {
        entry.is_complete = true;
    }

    // Update queue totals
    liq_queue.pending_liquidation = liq_queue.pending_liquidation.saturating_sub(aph_sold);

    // Update vault
    vault.locked_aph = vault.locked_aph.saturating_sub(aph_sold);
    vault.total_aph = vault.total_aph.saturating_sub(aph_sold);

    emit!(LiquidationExecuted {
        aph_sold,
        usdc_received,
        slippage_bps,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Reset circuit breaker (authority only)
#[derive(Accounts)]
pub struct ResetCircuitBreaker<'info> {
    #[account(
        seeds = [StakingConfig::SEED_PREFIX],
        bump = staking_config.bump,
    )]
    pub staking_config: Account<'info, StakingConfig>,

    #[account(
        mut,
        seeds = [LiquidationQueue::SEED_PREFIX],
        bump = liquidation_queue.bump,
    )]
    pub liquidation_queue: Account<'info, LiquidationQueue>,

    #[account(
        constraint = authority.key() == staking_config.authority @ StakingError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn reset_circuit_breaker(ctx: Context<ResetCircuitBreaker>) -> Result<()> {
    ctx.accounts.liquidation_queue.is_paused = false;
    Ok(())
}

/// Clear completed liquidation entries
#[derive(Accounts)]
pub struct ClearCompletedLiquidations<'info> {
    #[account(
        mut,
        seeds = [LiquidationQueue::SEED_PREFIX],
        bump = liquidation_queue.bump,
    )]
    pub liquidation_queue: Account<'info, LiquidationQueue>,
}

pub fn clear_completed_liquidations(ctx: Context<ClearCompletedLiquidations>) -> Result<()> {
    let queue = &mut ctx.accounts.liquidation_queue;
    queue.entries.retain(|e| !e.is_complete);
    Ok(())
}
