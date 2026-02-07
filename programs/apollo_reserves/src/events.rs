// programs/apollo_reserves/src/events.rs

use crate::state::WaterfallSource;
use anchor_lang::prelude::*;

/// Emitted when reserves are initialized
#[event]
pub struct ReservesInitialized {
    pub authority: Pubkey,
    pub usdc_mint: Pubkey,
    pub tier0_target_days: u16,
    pub tier1_target_days: u16,
    pub tier2_target_days: u16,
    pub timestamp: i64,
}

/// Emitted when vaults are created
#[event]
pub struct VaultsCreated {
    pub vault_authority: Pubkey,
    pub tier0_vault: Pubkey,
    pub tier1_vault: Pubkey,
    pub tier2_vault: Pubkey,
    pub runoff_vault: Pubkey,
    pub timestamp: i64,
}

/// Emitted when reserve targets are updated
#[event]
pub struct ReserveTargetsUpdated {
    pub old_tier0_days: u16,
    pub new_tier0_days: u16,
    pub old_tier1_days: u16,
    pub new_tier1_days: u16,
    pub old_tier2_days: u16,
    pub new_tier2_days: u16,
    pub updater: Pubkey,
    pub timestamp: i64,
}

/// Emitted when a contribution is routed to vaults
#[event]
pub struct ContributionRouted {
    pub member: Pubkey,
    pub total_amount: u64,
    pub to_tier0: u64,
    pub to_tier1: u64,
    pub to_tier2: u64,
    pub to_admin: u64,
    pub timestamp: i64,
}

/// Emitted when IBNR is computed/updated
#[event]
pub struct IbnrUpdated {
    pub old_ibnr: u64,
    pub new_ibnr: u64,
    pub avg_daily_claims: u64,
    pub reporting_lag_days: u16,
    pub development_factor_bps: u16,
    pub timestamp: i64,
}

/// Emitted when expected claims are updated
#[event]
pub struct ExpectedClaimsUpdated {
    pub old_expected_daily: u64,
    pub new_expected_daily: u64,
    pub updater: Pubkey,
    pub timestamp: i64,
}

/// Emitted when a claim payout occurs through waterfall
#[event]
pub struct ClaimPaidFromWaterfall {
    pub claim_id: u64,
    pub total_amount: u64,
    pub from_tier0: u64,
    pub from_tier1: u64,
    pub from_tier2: u64,
    pub from_staked: u64,
    pub recipient: Pubkey,
    pub timestamp: i64,
}

/// Emitted when run-off reserve is funded
#[event]
pub struct RunoffFunded {
    pub amount: u64,
    pub new_balance: u64,
    pub target_balance: u64,
    pub funder: Pubkey,
    pub timestamp: i64,
}

/// Emitted when run-off reserve is spent (emergency)
#[event]
pub struct RunoffSpent {
    pub amount: u64,
    pub new_balance: u64,
    pub reason: String,
    pub authorizer: Pubkey,
    pub timestamp: i64,
}

/// Emitted when reserves are refilled between tiers
#[event]
pub struct TierRefilled {
    pub from_tier: WaterfallSource,
    pub to_tier: WaterfallSource,
    pub amount: u64,
    pub timestamp: i64,
}

/// Emitted when coverage ratio changes significantly
#[event]
pub struct CoverageRatioChanged {
    pub old_ratio_bps: u16,
    pub new_ratio_bps: u16,
    pub target_ratio_bps: u16,
    pub min_ratio_bps: u16,
    pub timestamp: i64,
}

/// Emitted when reserve state snapshot is taken
#[event]
pub struct ReserveSnapshot {
    pub tier0_balance: u64,
    pub tier1_balance: u64,
    pub tier2_balance: u64,
    pub runoff_balance: u64,
    pub ibnr_usdc: u64,
    pub coverage_ratio_bps: u16,
    pub timestamp: i64,
}

/// Emitted when run-off mode is activated
#[event]
pub struct RunoffModeActivated {
    pub activator: Pubkey,
    pub target_balance: u64,
    pub current_balance: u64,
    pub timestamp: i64,
}
