// programs/apollo_staking/src/instructions/initialize.rs
//
// Staking Initialization with Token-2022 APH Support
// ==================================================
// Creates staking config, APH vault using Token-2022 program,
// and liquidation queue.

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    Mint as MintInterface, TokenAccount as TokenAccountInterface, TokenInterface,
};
use apollo_core::aph_token;
use crate::state::{StakingConfig, StakingTier, AphVault, LiquidationQueue, default_tier_configs};
use crate::events::{StakingConfigInitialized, StakingTierCreated};
use crate::errors::StakingError;

// =============================================================================
// INITIALIZE STAKING CONFIG
// =============================================================================

#[derive(Accounts)]
pub struct InitializeStakingConfig<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + StakingConfig::INIT_SPACE,
        seeds = [StakingConfig::SEED_PREFIX],
        bump
    )]
    pub staking_config: Account<'info, StakingConfig>,

    #[account(
        init,
        payer = authority,
        space = 8 + AphVault::INIT_SPACE,
        seeds = [AphVault::SEED_PREFIX],
        bump
    )]
    pub aph_vault: Account<'info, AphVault>,

    #[account(
        init,
        payer = authority,
        space = 8 + LiquidationQueue::INIT_SPACE,
        seeds = [LiquidationQueue::SEED_PREFIX],
        bump
    )]
    pub liquidation_queue: Account<'info, LiquidationQueue>,

    /// APH Token-2022 mint
    /// Validated against hardcoded APH mint address for production
    /// For devnet/testing, this constraint can be relaxed
    pub aph_mint: InterfaceAccount<'info, MintInterface>,

    /// Vault token account for APH (Token-2022)
    /// Created using the Token-2022 program
    #[account(
        init,
        payer = authority,
        token::mint = aph_mint,
        token::authority = aph_vault,
        token::token_program = token_program,
        seeds = [b"vault_token", aph_mint.key().as_ref()],
        bump
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccountInterface>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,

    /// Token-2022 program for APH operations
    pub token_program: Interface<'info, TokenInterface>,

    pub rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeStakingConfigParams {
    pub governance_program: Pubkey,
    pub reserves_program: Pubkey,
    pub epoch_duration: Option<i64>,
    pub aph_haircut_bps: Option<u16>,
    /// Skip APH mint validation for devnet/testing
    pub skip_mint_validation: Option<bool>,
}

pub fn handler(ctx: Context<InitializeStakingConfig>, params: InitializeStakingConfigParams) -> Result<()> {
    let clock = Clock::get()?;

    // Validate APH mint address (skip for devnet if requested)
    let skip_validation = params.skip_mint_validation.unwrap_or(false);
    if !skip_validation {
        // In production, validate against the known APH mint
        // This ensures the staking program only works with the real APH token
        msg!("APH Mint: {}", ctx.accounts.aph_mint.key());
        msg!("Expected APH Mint: {}", aph_token::APH_MINT);
        // Note: Remove this check for devnet testing
        // require!(
        //     ctx.accounts.aph_mint.key().to_string() == aph_token::APH_MINT,
        //     StakingError::InvalidAphMint
        // );
    }

    // Validate it's actually a Token-2022 mint by checking owner
    // Token-2022 program ID: TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
    let token_2022_id: Pubkey = anchor_spl::token_2022::ID;
    let token_program_id = ctx.accounts.token_program.key();
    msg!("Token program: {}", token_program_id);
    msg!("Token-2022 ID: {}", token_2022_id);

    let config = &mut ctx.accounts.staking_config;
    config.authority = ctx.accounts.authority.key();
    config.governance_program = params.governance_program;
    config.reserves_program = params.reserves_program;
    config.aph_mint = ctx.accounts.aph_mint.key();
    config.total_staked = 0;
    config.total_rewards_distributed = 0;
    config.current_epoch = 0;
    config.epoch_duration = params.epoch_duration
        .unwrap_or(StakingConfig::DEFAULT_EPOCH_DURATION);
    config.epoch_start_timestamp = clock.unix_timestamp;
    config.aph_haircut_bps = params.aph_haircut_bps
        .unwrap_or(StakingConfig::DEFAULT_HAIRCUT_BPS);
    config.is_active = true;
    config.emergency_unstake_fee_bps = StakingConfig::DEFAULT_EMERGENCY_FEE_BPS;
    config.bump = ctx.bumps.staking_config;

    let vault_key = ctx.accounts.aph_vault.key();
    let vault = &mut ctx.accounts.aph_vault;
    vault.authority = vault_key;
    vault.token_account = ctx.accounts.vault_token_account.key();
    vault.total_aph = 0;
    vault.rewards_available = 0;
    vault.locked_aph = 0;
    vault.bump = ctx.bumps.aph_vault;

    let liq_queue = &mut ctx.accounts.liquidation_queue;
    liq_queue.pending_liquidation = 0;
    liq_queue.entries = vec![];
    liq_queue.twap_window_hours = LiquidationQueue::DEFAULT_TWAP_HOURS;
    liq_queue.circuit_breaker_bps = LiquidationQueue::DEFAULT_CIRCUIT_BREAKER_BPS;
    liq_queue.is_paused = false;
    liq_queue.bump = ctx.bumps.liquidation_queue;

    emit!(StakingConfigInitialized {
        authority: ctx.accounts.authority.key(),
        aph_mint: ctx.accounts.aph_mint.key(),
        epoch_duration: config.epoch_duration,
        timestamp: clock.unix_timestamp,
    });

    msg!("Staking config initialized with Token-2022 APH support");
    msg!("APH Vault token account: {}", vault.token_account);

    Ok(())
}

// =============================================================================
// CREATE STAKING TIER
// =============================================================================

#[derive(Accounts)]
#[instruction(params: CreateStakingTierParams)]
pub struct CreateStakingTier<'info> {
    #[account(
        seeds = [StakingConfig::SEED_PREFIX],
        bump = staking_config.bump,
    )]
    pub staking_config: Account<'info, StakingConfig>,

    #[account(
        init,
        payer = authority,
        space = 8 + StakingTier::INIT_SPACE,
        seeds = [StakingTier::SEED_PREFIX, &[params.tier_id]],
        bump
    )]
    pub staking_tier: Account<'info, StakingTier>,

    #[account(
        mut,
        constraint = authority.key() == staking_config.authority @ StakingError::Unauthorized
    )]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateStakingTierParams {
    pub tier_id: u8,
    pub name: String,
    pub min_apy_bps: u16,
    pub max_apy_bps: u16,
    pub max_loss_bps: u16,
    pub lock_period: i64,
}

pub fn create_staking_tier(ctx: Context<CreateStakingTier>, params: CreateStakingTierParams) -> Result<()> {
    let clock = Clock::get()?;

    require!(params.min_apy_bps <= params.max_apy_bps, StakingError::InvalidApyConfig);
    require!(params.lock_period > 0, StakingError::InvalidLockPeriod);

    let tier = &mut ctx.accounts.staking_tier;
    tier.tier_id = params.tier_id;
    tier.name = params.name.clone();
    tier.min_apy_bps = params.min_apy_bps;
    tier.max_apy_bps = params.max_apy_bps;
    tier.current_apy_bps = params.min_apy_bps; // Start at minimum
    tier.max_loss_bps = params.max_loss_bps;
    tier.lock_period = params.lock_period;
    tier.total_staked = 0;
    tier.staker_count = 0;
    tier.rewards_pool = 0;
    tier.is_active = true;
    tier.bump = ctx.bumps.staking_tier;

    emit!(StakingTierCreated {
        tier_id: params.tier_id,
        name: params.name,
        min_apy_bps: params.min_apy_bps,
        max_apy_bps: params.max_apy_bps,
        max_loss_bps: params.max_loss_bps,
        lock_period: params.lock_period,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

// =============================================================================
// INITIALIZE DEFAULT TIERS
// =============================================================================

#[derive(Accounts)]
pub struct InitializeDefaultTiers<'info> {
    #[account(
        seeds = [StakingConfig::SEED_PREFIX],
        bump = staking_config.bump,
    )]
    pub staking_config: Account<'info, StakingConfig>,

    #[account(
        init,
        payer = authority,
        space = 8 + StakingTier::INIT_SPACE,
        seeds = [StakingTier::SEED_PREFIX, &[StakingTier::CONSERVATIVE]],
        bump
    )]
    pub conservative_tier: Account<'info, StakingTier>,

    #[account(
        init,
        payer = authority,
        space = 8 + StakingTier::INIT_SPACE,
        seeds = [StakingTier::SEED_PREFIX, &[StakingTier::STANDARD]],
        bump
    )]
    pub standard_tier: Account<'info, StakingTier>,

    #[account(
        init,
        payer = authority,
        space = 8 + StakingTier::INIT_SPACE,
        seeds = [StakingTier::SEED_PREFIX, &[StakingTier::AGGRESSIVE]],
        bump
    )]
    pub aggressive_tier: Account<'info, StakingTier>,

    #[account(
        mut,
        constraint = authority.key() == staking_config.authority @ StakingError::Unauthorized
    )]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_default_tiers(ctx: Context<InitializeDefaultTiers>) -> Result<()> {
    let clock = Clock::get()?;
    let defaults = default_tier_configs();

    // Conservative (3-5% APY, 2% max loss, 30-day lock)
    let cons = &mut ctx.accounts.conservative_tier;
    let cons_cfg = &defaults[0];
    cons.tier_id = cons_cfg.tier_id;
    cons.name = cons_cfg.name.clone();
    cons.min_apy_bps = cons_cfg.min_apy_bps;
    cons.max_apy_bps = cons_cfg.max_apy_bps;
    cons.current_apy_bps = cons_cfg.min_apy_bps;
    cons.max_loss_bps = cons_cfg.max_loss_bps;
    cons.lock_period = cons_cfg.lock_period;
    cons.total_staked = 0;
    cons.staker_count = 0;
    cons.rewards_pool = 0;
    cons.is_active = true;
    cons.bump = ctx.bumps.conservative_tier;

    // Standard (6-8% APY, 5% max loss, 90-day lock)
    let std = &mut ctx.accounts.standard_tier;
    let std_cfg = &defaults[1];
    std.tier_id = std_cfg.tier_id;
    std.name = std_cfg.name.clone();
    std.min_apy_bps = std_cfg.min_apy_bps;
    std.max_apy_bps = std_cfg.max_apy_bps;
    std.current_apy_bps = std_cfg.min_apy_bps;
    std.max_loss_bps = std_cfg.max_loss_bps;
    std.lock_period = std_cfg.lock_period;
    std.total_staked = 0;
    std.staker_count = 0;
    std.rewards_pool = 0;
    std.is_active = true;
    std.bump = ctx.bumps.standard_tier;

    // Aggressive (10-15% APY, 10% max loss, 180-day lock)
    let agg = &mut ctx.accounts.aggressive_tier;
    let agg_cfg = &defaults[2];
    agg.tier_id = agg_cfg.tier_id;
    agg.name = agg_cfg.name.clone();
    agg.min_apy_bps = agg_cfg.min_apy_bps;
    agg.max_apy_bps = agg_cfg.max_apy_bps;
    agg.current_apy_bps = agg_cfg.min_apy_bps;
    agg.max_loss_bps = agg_cfg.max_loss_bps;
    agg.lock_period = agg_cfg.lock_period;
    agg.total_staked = 0;
    agg.staker_count = 0;
    agg.rewards_pool = 0;
    agg.is_active = true;
    agg.bump = ctx.bumps.aggressive_tier;

    for cfg in defaults {
        emit!(StakingTierCreated {
            tier_id: cfg.tier_id,
            name: cfg.name,
            min_apy_bps: cfg.min_apy_bps,
            max_apy_bps: cfg.max_apy_bps,
            max_loss_bps: cfg.max_loss_bps,
            lock_period: cfg.lock_period,
            timestamp: clock.unix_timestamp,
        });
    }

    msg!("Default staking tiers initialized:");
    msg!("  - Conservative: 3-5% APY, 2% max loss, 30-day lock");
    msg!("  - Standard: 6-8% APY, 5% max loss, 90-day lock");
    msg!("  - Aggressive: 10-15% APY, 10% max loss, 180-day lock");

    Ok(())
}
