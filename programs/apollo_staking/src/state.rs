// programs/apollo_staking/src/state.rs

use anchor_lang::prelude::*;

/// Global staking configuration
/// PDA seeds: ["staking_config"]
#[account]
#[derive(InitSpace)]
pub struct StakingConfig {
    /// Authority (DAO)
    pub authority: Pubkey,

    /// Governance program
    pub governance_program: Pubkey,

    /// Reserves program (for eligible APH reporting)
    pub reserves_program: Pubkey,

    /// APH token mint
    pub aph_mint: Pubkey,

    /// Total APH staked across all tiers
    pub total_staked: u64,

    /// Total rewards distributed
    pub total_rewards_distributed: u64,

    /// Current epoch number
    pub current_epoch: u64,

    /// Epoch duration in seconds (default 7 days)
    pub epoch_duration: i64,

    /// Last epoch start timestamp
    pub epoch_start_timestamp: i64,

    /// Haircut percentage for APH in CAR calculation (basis points)
    /// Default 5000 = 50% haircut (only 50% of staked APH counts toward CAR)
    pub aph_haircut_bps: u16,

    /// Is staking active
    pub is_active: bool,

    /// Emergency unstake fee (basis points) - penalty for early exit
    pub emergency_unstake_fee_bps: u16,

    /// Bump seed
    pub bump: u8,
}

impl StakingConfig {
    pub const SEED_PREFIX: &'static [u8] = b"staking_config";
    pub const DEFAULT_EPOCH_DURATION: i64 = 7 * 24 * 60 * 60; // 7 days
    pub const DEFAULT_HAIRCUT_BPS: u16 = 5000; // 50%
    pub const DEFAULT_EMERGENCY_FEE_BPS: u16 = 1000; // 10%
}

/// Staking tier configuration
/// PDA seeds: ["staking_tier", tier_id]
#[account]
#[derive(InitSpace)]
pub struct StakingTier {
    /// Tier identifier (0 = Conservative, 1 = Standard, 2 = Aggressive)
    pub tier_id: u8,

    /// Tier name
    #[max_len(32)]
    pub name: String,

    /// Minimum APY in basis points
    pub min_apy_bps: u16,

    /// Maximum APY in basis points
    pub max_apy_bps: u16,

    /// Current APY in basis points
    pub current_apy_bps: u16,

    /// Maximum loss exposure in basis points
    /// Conservative: 200 (2%), Standard: 500 (5%), Aggressive: 1000 (10%)
    pub max_loss_bps: u16,

    /// Lock period in seconds
    pub lock_period: i64,

    /// Total staked in this tier
    pub total_staked: u64,

    /// Total stakers in this tier
    pub staker_count: u64,

    /// Rewards pool for this tier
    pub rewards_pool: u64,

    /// Is this tier active
    pub is_active: bool,

    /// Bump seed
    pub bump: u8,
}

impl StakingTier {
    pub const SEED_PREFIX: &'static [u8] = b"staking_tier";

    // Tier IDs
    pub const CONSERVATIVE: u8 = 0;
    pub const STANDARD: u8 = 1;
    pub const AGGRESSIVE: u8 = 2;
}

/// Default tier configurations
pub fn default_tier_configs() -> Vec<TierConfig> {
    vec![
        TierConfig {
            tier_id: StakingTier::CONSERVATIVE,
            name: "Conservative".to_string(),
            min_apy_bps: 300,   // 3%
            max_apy_bps: 500,   // 5%
            max_loss_bps: 200,  // 2%
            lock_period: 30 * 24 * 60 * 60, // 30 days
        },
        TierConfig {
            tier_id: StakingTier::STANDARD,
            name: "Standard".to_string(),
            min_apy_bps: 600,   // 6%
            max_apy_bps: 800,   // 8%
            max_loss_bps: 500,  // 5%
            lock_period: 90 * 24 * 60 * 60, // 90 days
        },
        TierConfig {
            tier_id: StakingTier::AGGRESSIVE,
            name: "Aggressive".to_string(),
            min_apy_bps: 1000,  // 10%
            max_apy_bps: 1500,  // 15%
            max_loss_bps: 1000, // 10%
            lock_period: 180 * 24 * 60 * 60, // 180 days
        },
    ]
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TierConfig {
    pub tier_id: u8,
    pub name: String,
    pub min_apy_bps: u16,
    pub max_apy_bps: u16,
    pub max_loss_bps: u16,
    pub lock_period: i64,
}

/// Individual staker position
/// PDA seeds: ["stake_position", staker, position_id]
#[account]
#[derive(InitSpace)]
pub struct StakePosition {
    /// Staker's wallet
    pub staker: Pubkey,

    /// Position ID (sequential per staker)
    pub position_id: u64,

    /// Tier this position is in
    pub tier_id: u8,

    /// Amount staked
    pub amount: u64,

    /// Original amount (before any slashing)
    pub original_amount: u64,

    /// Rewards earned (claimable)
    pub rewards_earned: u64,

    /// Rewards claimed
    pub rewards_claimed: u64,

    /// Stake timestamp
    pub staked_at: i64,

    /// Lock end timestamp
    pub lock_ends_at: i64,

    /// Last reward computation timestamp
    pub last_reward_at: i64,

    /// Is position active
    pub is_active: bool,

    /// Has been slashed
    pub was_slashed: bool,

    /// Slash amount (if any)
    pub slash_amount: u64,

    /// Bump seed
    pub bump: u8,
}

impl StakePosition {
    pub const SEED_PREFIX: &'static [u8] = b"stake_position";

    /// Check if position is unlocked
    pub fn is_unlocked(&self, current_time: i64) -> bool {
        current_time >= self.lock_ends_at
    }

    /// Get effective stake (after slashing)
    pub fn effective_stake(&self) -> u64 {
        self.amount.saturating_sub(self.slash_amount)
    }
}

/// Staker account (aggregates all positions)
/// PDA seeds: ["staker_account", staker]
#[account]
#[derive(InitSpace)]
pub struct StakerAccount {
    /// Staker's wallet
    pub staker: Pubkey,

    /// Total staked across all positions
    pub total_staked: u64,

    /// Total rewards earned
    pub total_rewards_earned: u64,

    /// Total rewards claimed
    pub total_rewards_claimed: u64,

    /// Number of active positions
    pub active_positions: u32,

    /// Next position ID
    pub next_position_id: u64,

    /// First stake timestamp
    pub first_stake_at: i64,

    /// Voting power (may differ from total_staked based on lock duration)
    pub voting_power: u64,

    /// Bump seed
    pub bump: u8,
}

impl StakerAccount {
    pub const SEED_PREFIX: &'static [u8] = b"staker_account";
}

/// APH vault for staked tokens
/// PDA seeds: ["aph_vault"]
#[account]
#[derive(InitSpace)]
pub struct AphVault {
    /// Vault authority (PDA)
    pub authority: Pubkey,

    /// APH token account
    pub token_account: Pubkey,

    /// Total APH in vault
    pub total_aph: u64,

    /// APH available for rewards
    pub rewards_available: u64,

    /// APH locked (staked)
    pub locked_aph: u64,

    /// Bump seed
    pub bump: u8,
}

impl AphVault {
    pub const SEED_PREFIX: &'static [u8] = b"aph_vault";
}

/// Liquidation queue for slashed APH
/// PDA seeds: ["liquidation_queue"]
#[account]
#[derive(InitSpace)]
pub struct LiquidationQueue {
    /// Total APH pending liquidation
    pub pending_liquidation: u64,

    /// Liquidation entries
    #[max_len(50)]
    pub entries: Vec<LiquidationEntry>,

    /// TWAP window (hours)
    pub twap_window_hours: u8,

    /// Circuit breaker slippage threshold (basis points)
    pub circuit_breaker_bps: u16,

    /// Is liquidation paused (circuit breaker triggered)
    pub is_paused: bool,

    /// Bump seed
    pub bump: u8,
}

impl LiquidationQueue {
    pub const SEED_PREFIX: &'static [u8] = b"liquidation_queue";
    pub const DEFAULT_TWAP_HOURS: u8 = 24; // 24 hour TWAP
    pub const DEFAULT_CIRCUIT_BREAKER_BPS: u16 = 1500; // 15% slippage triggers pause
}

/// Individual liquidation entry
#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct LiquidationEntry {
    /// APH amount to liquidate
    pub aph_amount: u64,
    /// Target USDC amount (at time of slash)
    pub target_usdc: u64,
    /// Created timestamp
    pub created_at: i64,
    /// Liquidation start timestamp
    pub start_at: i64,
    /// Liquidation end timestamp (TWAP window)
    pub end_at: i64,
    /// Amount already liquidated
    pub liquidated_aph: u64,
    /// USDC received
    pub received_usdc: u64,
    /// Is complete
    pub is_complete: bool,
}

/// Epoch snapshot for reward distribution
/// PDA seeds: ["epoch_snapshot", epoch_number]
#[account]
#[derive(InitSpace)]
pub struct EpochSnapshot {
    /// Epoch number
    pub epoch: u64,

    /// Epoch start timestamp
    pub start_timestamp: i64,

    /// Epoch end timestamp
    pub end_timestamp: i64,

    /// Total staked at epoch start
    pub total_staked: u64,

    /// Total rewards distributed this epoch
    pub rewards_distributed: u64,

    /// Average APY this epoch (basis points)
    pub average_apy_bps: u16,

    /// Was there a slash event this epoch
    pub had_slash_event: bool,

    /// Total slashed this epoch
    pub total_slashed: u64,

    /// Bump seed
    pub bump: u8,
}

impl EpochSnapshot {
    pub const SEED_PREFIX: &'static [u8] = b"epoch_snapshot";
}
