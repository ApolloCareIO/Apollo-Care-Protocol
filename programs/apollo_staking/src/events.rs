// programs/apollo_staking/src/events.rs

use anchor_lang::prelude::*;

/// Emitted when staking config is initialized
#[event]
pub struct StakingConfigInitialized {
    pub authority: Pubkey,
    pub aph_mint: Pubkey,
    pub epoch_duration: i64,
    pub timestamp: i64,
}

/// Emitted when a staking tier is created
#[event]
pub struct StakingTierCreated {
    pub tier_id: u8,
    pub name: String,
    pub min_apy_bps: u16,
    pub max_apy_bps: u16,
    pub max_loss_bps: u16,
    pub lock_period: i64,
    pub timestamp: i64,
}

/// Emitted when APH is staked
#[event]
pub struct Staked {
    pub staker: Pubkey,
    pub position_id: u64,
    pub tier_id: u8,
    pub amount: u64,
    pub lock_ends_at: i64,
    pub timestamp: i64,
}

/// Emitted when APH is unstaked
#[event]
pub struct Unstaked {
    pub staker: Pubkey,
    pub position_id: u64,
    pub amount: u64,
    pub rewards_claimed: u64,
    pub was_emergency: bool,
    pub fee_paid: u64,
    pub timestamp: i64,
}

/// Emitted when rewards are claimed
#[event]
pub struct RewardsClaimed {
    pub staker: Pubkey,
    pub position_id: u64,
    pub amount: u64,
    pub timestamp: i64,
}

/// Emitted when rewards are computed
#[event]
pub struct RewardsComputed {
    pub staker: Pubkey,
    pub position_id: u64,
    pub rewards_added: u64,
    pub total_rewards: u64,
    pub apy_applied_bps: u16,
    pub timestamp: i64,
}

/// Emitted when a slash occurs
#[event]
pub struct Slashed {
    pub tier_id: u8,
    pub total_slashed: u64,
    pub positions_affected: u32,
    pub reason: String,
    pub timestamp: i64,
}

/// Emitted when a position is slashed
#[event]
pub struct PositionSlashed {
    pub staker: Pubkey,
    pub position_id: u64,
    pub slash_amount: u64,
    pub remaining_amount: u64,
    pub timestamp: i64,
}

/// Emitted when APH is queued for liquidation
#[event]
pub struct LiquidationQueued {
    pub aph_amount: u64,
    pub target_usdc: u64,
    pub twap_start: i64,
    pub twap_end: i64,
    pub timestamp: i64,
}

/// Emitted when liquidation is executed
#[event]
pub struct LiquidationExecuted {
    pub aph_sold: u64,
    pub usdc_received: u64,
    pub slippage_bps: u16,
    pub timestamp: i64,
}

/// Emitted when circuit breaker triggers
#[event]
pub struct CircuitBreakerTriggered {
    pub slippage_observed_bps: u16,
    pub threshold_bps: u16,
    pub timestamp: i64,
}

/// Emitted when new epoch starts
#[event]
pub struct EpochStarted {
    pub epoch: u64,
    pub start_timestamp: i64,
    pub total_staked: u64,
    pub timestamp: i64,
}

/// Emitted when epoch ends
#[event]
pub struct EpochEnded {
    pub epoch: u64,
    pub rewards_distributed: u64,
    pub average_apy_bps: u16,
    pub had_slash: bool,
    pub timestamp: i64,
}

/// Emitted when tier APY is updated
#[event]
pub struct TierApyUpdated {
    pub tier_id: u8,
    pub old_apy_bps: u16,
    pub new_apy_bps: u16,
    pub timestamp: i64,
}

/// Emitted when rewards are added to pool
#[event]
pub struct RewardsPoolFunded {
    pub tier_id: u8,
    pub amount: u64,
    pub total_pool: u64,
    pub timestamp: i64,
}

/// Emitted when eligible APH is reported to reserves
#[event]
pub struct EligibleAphReported {
    pub total_staked: u64,
    pub haircut_bps: u16,
    pub eligible_usdc_value: u64,
    pub timestamp: i64,
}
