// programs/apollo_staking/src/lib.rs
//
// Apollo Staking Program
// ======================
// Three-tier APH staking system with:
// - Conservative (3-5% APY, 2% max loss, 30-day lock)
// - Standard (6-8% APY, 5% max loss, 90-day lock)
// - Aggressive (10-15% APY, 10% max loss, 180-day lock)
//
// Features:
// - Position-based staking with lock periods
// - Reward computation and claiming
// - Tier-based slashing for claim shortfalls
// - TWAP liquidation with circuit breaker protection
// - Eligible APH reporting for CAR calculations

use anchor_lang::prelude::*;

pub mod state;
pub mod errors;
pub mod events;
pub mod instructions;

use instructions::*;

declare_id!("AiJ1Gs6uGeoH5RXYUAZLZGyCotDFTRFUB3krQzrp3r5C");

#[program]
pub mod apollo_staking {
    use super::*;

    // ==================== INITIALIZATION ====================

    /// Initialize staking configuration
    pub fn initialize_staking_config(
        ctx: Context<InitializeStakingConfig>,
        params: InitializeStakingConfigParams,
    ) -> Result<()> {
        instructions::initialize::handler(ctx, params)
    }

    /// Create a staking tier
    pub fn create_staking_tier(
        ctx: Context<CreateStakingTier>,
        params: CreateStakingTierParams,
    ) -> Result<()> {
        instructions::initialize::create_staking_tier(ctx, params)
    }

    /// Initialize all default tiers (Conservative, Standard, Aggressive)
    pub fn initialize_default_tiers(ctx: Context<InitializeDefaultTiers>) -> Result<()> {
        instructions::initialize::initialize_default_tiers(ctx)
    }

    // ==================== STAKING ====================

    /// Stake APH tokens into a tier
    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        instructions::staking::stake(ctx, amount)
    }

    /// Unstake APH tokens (after lock period)
    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        instructions::staking::unstake(ctx)
    }

    /// Emergency unstake (before lock period, with fee)
    pub fn emergency_unstake(ctx: Context<EmergencyUnstake>) -> Result<()> {
        instructions::staking::emergency_unstake(ctx)
    }

    // ==================== REWARDS ====================

    /// Compute rewards for a position
    pub fn compute_rewards(ctx: Context<ComputeRewards>) -> Result<()> {
        instructions::rewards::compute_rewards(ctx)
    }

    /// Claim rewards without unstaking
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        instructions::rewards::claim_rewards(ctx)
    }

    /// Fund rewards pool for a tier
    pub fn fund_rewards_pool(ctx: Context<FundRewardsPool>, amount: u64) -> Result<()> {
        instructions::rewards::fund_rewards_pool(ctx, amount)
    }

    /// Update tier APY
    pub fn update_tier_apy(ctx: Context<UpdateTierApy>, new_apy_bps: u16) -> Result<()> {
        instructions::rewards::update_tier_apy(ctx, new_apy_bps)
    }

    // ==================== SLASHING ====================

    /// Slash a specific position
    pub fn slash_position(
        ctx: Context<SlashPosition>,
        slash_amount: u64,
        target_usdc_value: u64,
        reason: String,
    ) -> Result<()> {
        instructions::slashing::slash_position(ctx, slash_amount, target_usdc_value, reason)
    }

    /// Slash across entire tier (proportional)
    pub fn slash_tier(
        ctx: Context<SlashTier>,
        total_slash_amount: u64,
        target_usdc_value: u64,
        reason: String,
    ) -> Result<()> {
        instructions::slashing::slash_tier(ctx, total_slash_amount, target_usdc_value, reason)
    }

    /// Execute liquidation (sell slashed APH for USDC)
    pub fn execute_liquidation(
        ctx: Context<ExecuteLiquidation>,
        entry_index: u8,
        aph_sold: u64,
        usdc_received: u64,
    ) -> Result<()> {
        instructions::slashing::execute_liquidation(ctx, entry_index, aph_sold, usdc_received)
    }

    /// Reset circuit breaker
    pub fn reset_circuit_breaker(ctx: Context<ResetCircuitBreaker>) -> Result<()> {
        instructions::slashing::reset_circuit_breaker(ctx)
    }

    /// Clear completed liquidation entries
    pub fn clear_completed_liquidations(ctx: Context<ClearCompletedLiquidations>) -> Result<()> {
        instructions::slashing::clear_completed_liquidations(ctx)
    }
}

/// Public helpers for CPI
pub mod staking_helpers {
    use super::*;

    pub fn get_staking_config_seeds() -> &'static [&'static [u8]] {
        &[state::StakingConfig::SEED_PREFIX]
    }

    pub fn get_aph_vault_seeds() -> &'static [&'static [u8]] {
        &[state::AphVault::SEED_PREFIX]
    }

    pub fn get_liquidation_queue_seeds() -> &'static [&'static [u8]] {
        &[state::LiquidationQueue::SEED_PREFIX]
    }
}
