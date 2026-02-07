// programs/apollo_staking/src/instructions/staking.rs
//
// APH Token-2022 Staking Operations
// =================================
// Uses token_interface for Token-2022 compatibility with APH token.
// Handles transfer fee extension awareness.

use crate::errors::StakingError;
use crate::events::{Staked, Unstaked};
use crate::state::{AphVault, StakePosition, StakerAccount, StakingConfig, StakingTier};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    self, Mint as MintInterface, TokenAccount as TokenAccountInterface, TokenInterface,
    TransferChecked,
};

// =============================================================================
// STAKE APH TOKENS
// =============================================================================

/// Stake APH tokens into a tier
/// Uses Token-2022 transfer_checked for APH
#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(
        mut,
        seeds = [StakingConfig::SEED_PREFIX],
        bump = staking_config.bump,
        constraint = staking_config.is_active @ StakingError::StakingPaused
    )]
    pub staking_config: Account<'info, StakingConfig>,

    #[account(
        mut,
        seeds = [StakingTier::SEED_PREFIX, &[staking_tier.tier_id]],
        bump = staking_tier.bump,
        constraint = staking_tier.is_active @ StakingError::TierNotActive
    )]
    pub staking_tier: Account<'info, StakingTier>,

    #[account(
        mut,
        seeds = [AphVault::SEED_PREFIX],
        bump = aph_vault.bump,
    )]
    pub aph_vault: Account<'info, AphVault>,

    #[account(
        init_if_needed,
        payer = staker,
        space = 8 + StakerAccount::INIT_SPACE,
        seeds = [StakerAccount::SEED_PREFIX, staker.key().as_ref()],
        bump
    )]
    pub staker_account: Account<'info, StakerAccount>,

    #[account(
        init,
        payer = staker,
        space = 8 + StakePosition::INIT_SPACE,
        seeds = [
            StakePosition::SEED_PREFIX,
            staker.key().as_ref(),
            &staker_account.next_position_id.to_le_bytes()
        ],
        bump
    )]
    pub stake_position: Account<'info, StakePosition>,

    /// APH Token-2022 mint
    #[account(
        constraint = aph_mint.key() == staking_config.aph_mint @ StakingError::InvalidTokenAccount
    )]
    pub aph_mint: InterfaceAccount<'info, MintInterface>,

    /// Staker's APH token account (Token-2022)
    #[account(
        mut,
        constraint = staker_token_account.mint == staking_config.aph_mint @ StakingError::InvalidTokenAccount,
        constraint = staker_token_account.owner == staker.key() @ StakingError::Unauthorized
    )]
    pub staker_token_account: InterfaceAccount<'info, TokenAccountInterface>,

    /// Vault's APH token account (Token-2022)
    #[account(
        mut,
        constraint = vault_token_account.key() == aph_vault.token_account @ StakingError::InvalidTokenAccount
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccountInterface>,

    #[account(mut)]
    pub staker: Signer<'info>,

    pub system_program: Program<'info, System>,

    /// Token-2022 program for APH operations
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
    let clock = Clock::get()?;

    require!(amount > 0, StakingError::InsufficientStakeAmount);

    // Get APH decimals for transfer_checked
    let decimals = ctx.accounts.aph_mint.decimals;

    // Transfer APH to vault using Token-2022 transfer_checked
    // This properly handles transfer fees if enabled
    token_interface::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.staker_token_account.to_account_info(),
                mint: ctx.accounts.aph_mint.to_account_info(),
                to: ctx.accounts.vault_token_account.to_account_info(),
                authority: ctx.accounts.staker.to_account_info(),
            },
        ),
        amount,
        decimals,
    )?;

    let tier = &mut ctx.accounts.staking_tier;
    let config = &mut ctx.accounts.staking_config;
    let vault = &mut ctx.accounts.aph_vault;
    let staker_account = &mut ctx.accounts.staker_account;
    let position = &mut ctx.accounts.stake_position;

    // Calculate lock end
    let lock_ends_at = clock.unix_timestamp + tier.lock_period;

    // Initialize position
    let position_id = staker_account.next_position_id;
    position.staker = ctx.accounts.staker.key();
    position.position_id = position_id;
    position.tier_id = tier.tier_id;
    position.amount = amount;
    position.original_amount = amount;
    position.rewards_earned = 0;
    position.rewards_claimed = 0;
    position.staked_at = clock.unix_timestamp;
    position.lock_ends_at = lock_ends_at;
    position.last_reward_at = clock.unix_timestamp;
    position.is_active = true;
    position.was_slashed = false;
    position.slash_amount = 0;
    position.bump = ctx.bumps.stake_position;

    // Update staker account
    if staker_account.total_staked == 0 {
        staker_account.staker = ctx.accounts.staker.key();
        staker_account.first_stake_at = clock.unix_timestamp;
        staker_account.bump = ctx.bumps.staker_account;
    }
    staker_account.total_staked = staker_account.total_staked.saturating_add(amount);
    staker_account.active_positions += 1;
    staker_account.next_position_id += 1;
    staker_account.voting_power = staker_account.voting_power.saturating_add(amount);

    // Update tier
    tier.total_staked = tier.total_staked.saturating_add(amount);
    tier.staker_count += 1;

    // Update config
    config.total_staked = config.total_staked.saturating_add(amount);

    // Update vault
    vault.total_aph = vault.total_aph.saturating_add(amount);
    vault.locked_aph = vault.locked_aph.saturating_add(amount);

    emit!(Staked {
        staker: ctx.accounts.staker.key(),
        position_id,
        tier_id: tier.tier_id,
        amount,
        lock_ends_at,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

// =============================================================================
// UNSTAKE APH TOKENS (After Lock Period)
// =============================================================================

/// Unstake APH tokens (after lock period)
#[derive(Accounts)]
pub struct Unstake<'info> {
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

    /// Staker's APH token account (Token-2022)
    #[account(
        mut,
        constraint = staker_token_account.mint == staking_config.aph_mint @ StakingError::InvalidTokenAccount,
        constraint = staker_token_account.owner == staker.key() @ StakingError::Unauthorized
    )]
    pub staker_token_account: InterfaceAccount<'info, TokenAccountInterface>,

    /// Vault's APH token account (Token-2022)
    #[account(
        mut,
        constraint = vault_token_account.key() == aph_vault.token_account @ StakingError::InvalidTokenAccount
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccountInterface>,

    pub staker: Signer<'info>,

    /// Token-2022 program for APH operations
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
    let clock = Clock::get()?;
    let position = &mut ctx.accounts.stake_position;

    // Check lock period is complete
    require!(
        position.is_unlocked(clock.unix_timestamp),
        StakingError::PositionLocked
    );

    let effective_amount = position.effective_stake();
    let rewards = position
        .rewards_earned
        .saturating_sub(position.rewards_claimed);
    let total_withdrawal = effective_amount.saturating_add(rewards);

    // Get decimals for transfer_checked
    let decimals = ctx.accounts.aph_mint.decimals;

    // Transfer APH back to staker using vault authority PDA
    let vault_seeds = &[AphVault::SEED_PREFIX, &[ctx.accounts.aph_vault.bump]];
    let signer_seeds = &[&vault_seeds[..]];

    token_interface::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.vault_token_account.to_account_info(),
                mint: ctx.accounts.aph_mint.to_account_info(),
                to: ctx.accounts.staker_token_account.to_account_info(),
                authority: ctx.accounts.aph_vault.to_account_info(),
            },
            signer_seeds,
        ),
        total_withdrawal,
        decimals,
    )?;

    // Update state
    let tier = &mut ctx.accounts.staking_tier;
    let config = &mut ctx.accounts.staking_config;
    let vault = &mut ctx.accounts.aph_vault;
    let staker_account = &mut ctx.accounts.staker_account;

    // Mark position closed
    position.is_active = false;
    position.rewards_claimed = position.rewards_earned;

    // Update staker account
    staker_account.total_staked = staker_account
        .total_staked
        .saturating_sub(position.original_amount);
    staker_account.active_positions = staker_account.active_positions.saturating_sub(1);
    staker_account.total_rewards_claimed =
        staker_account.total_rewards_claimed.saturating_add(rewards);
    staker_account.voting_power = staker_account
        .voting_power
        .saturating_sub(position.original_amount);

    // Update tier
    tier.total_staked = tier.total_staked.saturating_sub(position.original_amount);
    tier.staker_count = tier.staker_count.saturating_sub(1);

    // Update config
    config.total_staked = config.total_staked.saturating_sub(position.original_amount);
    config.total_rewards_distributed = config.total_rewards_distributed.saturating_add(rewards);

    // Update vault
    vault.total_aph = vault.total_aph.saturating_sub(total_withdrawal);
    vault.locked_aph = vault.locked_aph.saturating_sub(effective_amount);

    emit!(Unstaked {
        staker: ctx.accounts.staker.key(),
        position_id: position.position_id,
        amount: effective_amount,
        rewards_claimed: rewards,
        was_emergency: false,
        fee_paid: 0,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

// =============================================================================
// EMERGENCY UNSTAKE (Before Lock Period - With Fee)
// =============================================================================

/// Emergency unstake (before lock period, with fee)
#[derive(Accounts)]
pub struct EmergencyUnstake<'info> {
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

pub fn emergency_unstake(ctx: Context<EmergencyUnstake>) -> Result<()> {
    let clock = Clock::get()?;
    let config = &ctx.accounts.staking_config;
    let position = &mut ctx.accounts.stake_position;

    let effective_amount = position.effective_stake();

    // Calculate emergency fee
    let fee = effective_amount
        .saturating_mul(config.emergency_unstake_fee_bps as u64)
        .checked_div(10000)
        .unwrap_or(0);

    let withdrawal_amount = effective_amount.saturating_sub(fee);
    // No rewards for emergency unstake

    // Get decimals for transfer_checked
    let decimals = ctx.accounts.aph_mint.decimals;

    // Transfer APH back to staker (minus fee)
    let vault_seeds = &[AphVault::SEED_PREFIX, &[ctx.accounts.aph_vault.bump]];
    let signer_seeds = &[&vault_seeds[..]];

    token_interface::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.vault_token_account.to_account_info(),
                mint: ctx.accounts.aph_mint.to_account_info(),
                to: ctx.accounts.staker_token_account.to_account_info(),
                authority: ctx.accounts.aph_vault.to_account_info(),
            },
            signer_seeds,
        ),
        withdrawal_amount,
        decimals,
    )?;

    // Update state
    let tier = &mut ctx.accounts.staking_tier;
    let config = &mut ctx.accounts.staking_config;
    let vault = &mut ctx.accounts.aph_vault;
    let staker_account = &mut ctx.accounts.staker_account;

    // Mark position closed
    position.is_active = false;

    // Update staker account
    staker_account.total_staked = staker_account
        .total_staked
        .saturating_sub(position.original_amount);
    staker_account.active_positions = staker_account.active_positions.saturating_sub(1);
    staker_account.voting_power = staker_account
        .voting_power
        .saturating_sub(position.original_amount);

    // Update tier - fee stays in rewards pool
    tier.total_staked = tier.total_staked.saturating_sub(position.original_amount);
    tier.staker_count = tier.staker_count.saturating_sub(1);
    tier.rewards_pool = tier.rewards_pool.saturating_add(fee);

    // Update config
    config.total_staked = config.total_staked.saturating_sub(position.original_amount);

    // Update vault
    vault.total_aph = vault.total_aph.saturating_sub(withdrawal_amount);
    vault.locked_aph = vault.locked_aph.saturating_sub(effective_amount);
    vault.rewards_available = vault.rewards_available.saturating_add(fee);

    emit!(Unstaked {
        staker: ctx.accounts.staker.key(),
        position_id: position.position_id,
        amount: withdrawal_amount,
        rewards_claimed: 0,
        was_emergency: true,
        fee_paid: fee,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
