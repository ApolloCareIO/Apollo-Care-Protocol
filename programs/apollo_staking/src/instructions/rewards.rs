// programs/apollo_staking/src/instructions/rewards.rs
//
// Staking Rewards with Token-2022 APH Support
// ============================================
// Reward computation, claiming, and pool funding using Token-2022.

use crate::errors::StakingError;
use crate::events::{RewardsClaimed, RewardsComputed, RewardsPoolFunded, TierApyUpdated};
use crate::state::{AphVault, StakePosition, StakerAccount, StakingConfig, StakingTier};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    self, Mint as MintInterface, TokenAccount as TokenAccountInterface, TokenInterface,
    TransferChecked,
};

// =============================================================================
// COMPUTE REWARDS
// =============================================================================

/// Compute rewards for a position
#[derive(Accounts)]
pub struct ComputeRewards<'info> {
    #[account(
        seeds = [StakingConfig::SEED_PREFIX],
        bump = staking_config.bump,
    )]
    pub staking_config: Account<'info, StakingConfig>,

    #[account(
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
        seeds = [StakerAccount::SEED_PREFIX, stake_position.staker.as_ref()],
        bump = staker_account.bump,
    )]
    pub staker_account: Account<'info, StakerAccount>,
}

pub fn compute_rewards(ctx: Context<ComputeRewards>) -> Result<()> {
    let clock = Clock::get()?;
    let tier = &ctx.accounts.staking_tier;
    let position = &mut ctx.accounts.stake_position;
    let staker_account = &mut ctx.accounts.staker_account;

    // Calculate time since last reward computation
    let time_elapsed = clock.unix_timestamp - position.last_reward_at;
    if time_elapsed <= 0 {
        return Ok(());
    }

    let effective_stake = position.effective_stake();
    if effective_stake == 0 {
        return Ok(());
    }

    // Calculate rewards: (stake * apy * time) / (365 days * 10000 bps)
    // Using safe math to avoid overflow
    let seconds_per_year: u64 = 365 * 24 * 60 * 60;
    let apy_bps = tier.current_apy_bps as u64;

    let rewards = effective_stake
        .saturating_mul(apy_bps)
        .saturating_mul(time_elapsed as u64)
        .checked_div(seconds_per_year)
        .unwrap_or(0)
        .checked_div(10000)
        .unwrap_or(0);

    position.rewards_earned = position.rewards_earned.saturating_add(rewards);
    position.last_reward_at = clock.unix_timestamp;

    staker_account.total_rewards_earned =
        staker_account.total_rewards_earned.saturating_add(rewards);

    emit!(RewardsComputed {
        staker: position.staker,
        position_id: position.position_id,
        rewards_added: rewards,
        total_rewards: position.rewards_earned,
        apy_applied_bps: tier.current_apy_bps,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

// =============================================================================
// CLAIM REWARDS
// =============================================================================

/// Claim rewards without unstaking
#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(
        mut,
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
        seeds = [AphVault::SEED_PREFIX],
        bump = aph_vault.bump,
    )]
    pub aph_vault: Account<'info, AphVault>,

    #[account(
        mut,
        seeds = [StakerAccount::SEED_PREFIX, staker.key().as_ref()],
        bump = staker_account.bump,
    )]
    pub staker_account: Account<'info, StakerAccount>,

    #[account(
        mut,
        seeds = [
            StakePosition::SEED_PREFIX,
            staker.key().as_ref(),
            &stake_position.position_id.to_le_bytes()
        ],
        bump = stake_position.bump,
        constraint = stake_position.staker == staker.key() @ StakingError::Unauthorized,
        constraint = stake_position.is_active @ StakingError::PositionAlreadyClosed
    )]
    pub stake_position: Account<'info, StakePosition>,

    /// APH Token-2022 mint
    #[account(
        constraint = aph_mint.key() == staking_config.aph_mint @ StakingError::InvalidTokenAccount
    )]
    pub aph_mint: InterfaceAccount<'info, MintInterface>,

    #[account(
        mut,
        constraint = staker_token_account.mint == staking_config.aph_mint @ StakingError::InvalidTokenAccount,
        constraint = staker_token_account.owner == staker.key() @ StakingError::Unauthorized
    )]
    pub staker_token_account: InterfaceAccount<'info, TokenAccountInterface>,

    #[account(
        mut,
        constraint = vault_token_account.key() == aph_vault.token_account @ StakingError::InvalidTokenAccount
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccountInterface>,

    pub staker: Signer<'info>,

    /// Token-2022 program for APH operations
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
    let clock = Clock::get()?;
    // Get account infos before mutable borrows
    let vault_account_info = ctx.accounts.aph_vault.to_account_info();

    let position = &mut ctx.accounts.stake_position;
    let tier = &mut ctx.accounts.staking_tier;
    let vault = &mut ctx.accounts.aph_vault;
    let staker_account = &mut ctx.accounts.staker_account;
    let config = &mut ctx.accounts.staking_config;

    let claimable = position
        .rewards_earned
        .saturating_sub(position.rewards_claimed);
    require!(claimable > 0, StakingError::NoRewardsToClaim);
    require!(
        tier.rewards_pool >= claimable,
        StakingError::InsufficientRewardsPool
    );

    // Get decimals for transfer_checked
    let decimals = ctx.accounts.aph_mint.decimals;

    // Transfer rewards from vault to staker using Token-2022
    let vault_seeds = &[AphVault::SEED_PREFIX, &[vault.bump]];
    let signer_seeds = &[&vault_seeds[..]];

    token_interface::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.vault_token_account.to_account_info(),
                mint: ctx.accounts.aph_mint.to_account_info(),
                to: ctx.accounts.staker_token_account.to_account_info(),
                authority: vault_account_info.clone(),
            },
            signer_seeds,
        ),
        claimable,
        decimals,
    )?;

    // Update state
    position.rewards_claimed = position.rewards_earned;
    staker_account.total_rewards_claimed = staker_account
        .total_rewards_claimed
        .saturating_add(claimable);
    tier.rewards_pool = tier.rewards_pool.saturating_sub(claimable);
    vault.rewards_available = vault.rewards_available.saturating_sub(claimable);
    vault.total_aph = vault.total_aph.saturating_sub(claimable);
    config.total_rewards_distributed = config.total_rewards_distributed.saturating_add(claimable);

    emit!(RewardsClaimed {
        staker: ctx.accounts.staker.key(),
        position_id: position.position_id,
        amount: claimable,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

// =============================================================================
// FUND REWARDS POOL
// =============================================================================

/// Fund rewards pool for a tier
#[derive(Accounts)]
pub struct FundRewardsPool<'info> {
    #[account(
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
        seeds = [AphVault::SEED_PREFIX],
        bump = aph_vault.bump,
    )]
    pub aph_vault: Account<'info, AphVault>,

    /// APH Token-2022 mint
    #[account(
        constraint = aph_mint.key() == staking_config.aph_mint @ StakingError::InvalidTokenAccount
    )]
    pub aph_mint: InterfaceAccount<'info, MintInterface>,

    #[account(
        mut,
        constraint = funder_token_account.mint == staking_config.aph_mint @ StakingError::InvalidTokenAccount
    )]
    pub funder_token_account: InterfaceAccount<'info, TokenAccountInterface>,

    #[account(
        mut,
        constraint = vault_token_account.key() == aph_vault.token_account @ StakingError::InvalidTokenAccount
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccountInterface>,

    pub funder: Signer<'info>,

    /// Token-2022 program for APH operations
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn fund_rewards_pool(ctx: Context<FundRewardsPool>, amount: u64) -> Result<()> {
    let clock = Clock::get()?;

    require!(amount > 0, StakingError::InsufficientStakeAmount);

    // Get decimals for transfer_checked
    let decimals = ctx.accounts.aph_mint.decimals;

    // Transfer APH to vault using Token-2022
    token_interface::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.funder_token_account.to_account_info(),
                mint: ctx.accounts.aph_mint.to_account_info(),
                to: ctx.accounts.vault_token_account.to_account_info(),
                authority: ctx.accounts.funder.to_account_info(),
            },
        ),
        amount,
        decimals,
    )?;

    let tier = &mut ctx.accounts.staking_tier;
    let vault = &mut ctx.accounts.aph_vault;

    tier.rewards_pool = tier.rewards_pool.saturating_add(amount);
    vault.total_aph = vault.total_aph.saturating_add(amount);
    vault.rewards_available = vault.rewards_available.saturating_add(amount);

    emit!(RewardsPoolFunded {
        tier_id: tier.tier_id,
        amount,
        total_pool: tier.rewards_pool,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

// =============================================================================
// UPDATE TIER APY
// =============================================================================

/// Update tier APY
#[derive(Accounts)]
pub struct UpdateTierApy<'info> {
    #[account(
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
        constraint = authority.key() == staking_config.authority @ StakingError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn update_tier_apy(ctx: Context<UpdateTierApy>, new_apy_bps: u16) -> Result<()> {
    let clock = Clock::get()?;
    let tier = &mut ctx.accounts.staking_tier;

    // Validate within bounds
    require!(
        new_apy_bps >= tier.min_apy_bps && new_apy_bps <= tier.max_apy_bps,
        StakingError::InvalidApyConfig
    );

    let old_apy = tier.current_apy_bps;
    tier.current_apy_bps = new_apy_bps;

    emit!(TierApyUpdated {
        tier_id: tier.tier_id,
        old_apy_bps: old_apy,
        new_apy_bps,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
