// programs/apollo_staking/src/errors.rs

use anchor_lang::prelude::*;

#[error_code]
pub enum StakingError {
    #[msg("Staking system not initialized")]
    NotInitialized,

    #[msg("Staking is currently paused")]
    StakingPaused,

    #[msg("Unauthorized: caller lacks permission")]
    Unauthorized,

    #[msg("Invalid staking tier")]
    InvalidTier,

    #[msg("Tier is not active")]
    TierNotActive,

    #[msg("Insufficient stake amount")]
    InsufficientStakeAmount,

    #[msg("Position is still locked")]
    PositionLocked,

    #[msg("Position not found")]
    PositionNotFound,

    #[msg("Position already closed")]
    PositionAlreadyClosed,

    #[msg("No rewards to claim")]
    NoRewardsToClaim,

    #[msg("Insufficient rewards in pool")]
    InsufficientRewardsPool,

    #[msg("Invalid lock period")]
    InvalidLockPeriod,

    #[msg("Invalid APY configuration")]
    InvalidApyConfig,

    #[msg("Epoch not complete")]
    EpochNotComplete,

    #[msg("Slash amount exceeds position")]
    SlashExceedsPosition,

    #[msg("Liquidation queue is full")]
    LiquidationQueueFull,

    #[msg("Liquidation circuit breaker triggered")]
    CircuitBreakerTriggered,

    #[msg("TWAP window not complete")]
    TwapWindowNotComplete,

    #[msg("Slippage exceeds threshold")]
    SlippageExceeded,

    #[msg("Emergency unstake fee required")]
    EmergencyFeeRequired,

    #[msg("Math overflow")]
    MathOverflow,

    #[msg("Invalid token account")]
    InvalidTokenAccount,

    #[msg("Vault balance insufficient")]
    VaultInsufficient,

    #[msg("Invalid APH Token-2022 mint")]
    InvalidAphMint,

    #[msg("Token-2022 operation failed")]
    Token2022Error,
}
