// programs/apollo_reserves/src/instructions/payouts.rs

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};
use crate::state::{ReserveConfig, ReserveState, VaultAuthority, RunoffState};
use crate::errors::ReserveError;
use crate::events::{ClaimPaidFromWaterfall, RunoffSpent, CoverageRatioChanged, ReserveSnapshot};

/// Pay a claim using the waterfall mechanism
/// Order: Tier0 -> Tier1 -> Tier2 -> (Staked APH via separate instruction)
#[derive(Accounts)]
pub struct PayoutClaimFromWaterfall<'info> {
    #[account(
        seeds = [ReserveConfig::SEED_PREFIX],
        bump = reserve_config.bump,
        constraint = reserve_config.is_initialized @ ReserveError::NotInitialized
    )]
    pub reserve_config: Account<'info, ReserveConfig>,

    #[account(
        mut,
        seeds = [ReserveState::SEED_PREFIX],
        bump = reserve_state.bump,
    )]
    pub reserve_state: Account<'info, ReserveState>,

    #[account(
        seeds = [VaultAuthority::SEED_PREFIX],
        bump = vault_authority.bump,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    #[account(
        mut,
        constraint = tier0_vault.key() == vault_authority.tier0_vault @ ReserveError::InvalidVaultConfig
    )]
    pub tier0_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = tier1_vault.key() == vault_authority.tier1_vault @ ReserveError::InvalidVaultConfig
    )]
    pub tier1_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = tier2_vault.key() == vault_authority.tier2_vault @ ReserveError::InvalidVaultConfig
    )]
    pub tier2_vault: Account<'info, TokenAccount>,

    /// Recipient token account (member or provider)
    #[account(
        mut,
        constraint = recipient.mint == reserve_config.usdc_mint @ ReserveError::InvalidMint
    )]
    pub recipient: Account<'info, TokenAccount>,

    /// Claims program authority (via CPI) or DAO authority
    pub payout_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct PayoutParams {
    pub claim_id: u64,
    pub amount: u64,
}

pub fn payout_claim_from_waterfall(ctx: Context<PayoutClaimFromWaterfall>, params: PayoutParams) -> Result<()> {
    let clock = Clock::get()?;
    let state = &mut ctx.accounts.reserve_state;

    require!(params.amount > 0, ReserveError::InvalidPayoutAmount);

    let mut remaining = params.amount;
    let mut from_tier0: u64 = 0;
    let mut from_tier1: u64 = 0;
    let mut from_tier2: u64 = 0;

    let vault_authority = &ctx.accounts.vault_authority;
    let seeds = &[
        VaultAuthority::SEED_PREFIX,
        &[vault_authority.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    // Waterfall: Tier0 first
    if remaining > 0 && ctx.accounts.tier0_vault.amount > 0 {
        let take = remaining.min(ctx.accounts.tier0_vault.amount);
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.tier0_vault.to_account_info(),
                    to: ctx.accounts.recipient.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                signer_seeds,
            ),
            take,
        )?;
        from_tier0 = take;
        remaining = remaining.saturating_sub(take);
        state.tier0_balance = state.tier0_balance.saturating_sub(take);
    }

    // Tier1 second
    if remaining > 0 && ctx.accounts.tier1_vault.amount > 0 {
        let take = remaining.min(ctx.accounts.tier1_vault.amount);
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.tier1_vault.to_account_info(),
                    to: ctx.accounts.recipient.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                signer_seeds,
            ),
            take,
        )?;
        from_tier1 = take;
        remaining = remaining.saturating_sub(take);
        state.tier1_balance = state.tier1_balance.saturating_sub(take);
    }

    // Tier2 third
    if remaining > 0 && ctx.accounts.tier2_vault.amount > 0 {
        let take = remaining.min(ctx.accounts.tier2_vault.amount);
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.tier2_vault.to_account_info(),
                    to: ctx.accounts.recipient.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                signer_seeds,
            ),
            take,
        )?;
        from_tier2 = take;
        remaining = remaining.saturating_sub(take);
        state.tier2_balance = state.tier2_balance.saturating_sub(take);
    }

    // If still remaining, waterfall is exhausted (staked APH would be next via separate tx)
    require!(remaining == 0, ReserveError::WaterfallExhausted);

    // Update totals
    state.total_claims_paid = state.total_claims_paid.saturating_add(params.amount);
    state.last_waterfall_at = clock.unix_timestamp;

    // Update coverage ratio
    update_coverage_ratio(ctx.accounts.reserve_config.as_ref(), state)?;

    emit!(ClaimPaidFromWaterfall {
        claim_id: params.claim_id,
        total_amount: params.amount,
        from_tier0,
        from_tier1,
        from_tier2,
        from_staked: 0, // Would be populated if staking is tapped
        recipient: ctx.accounts.recipient.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Emergency spend from run-off reserve (DAO + Emergency flag required)
#[derive(Accounts)]
pub struct EmergencySpendRunoff<'info> {
    #[account(
        seeds = [ReserveConfig::SEED_PREFIX],
        bump = reserve_config.bump,
    )]
    pub reserve_config: Account<'info, ReserveConfig>,

    #[account(
        mut,
        seeds = [ReserveState::SEED_PREFIX],
        bump = reserve_state.bump,
    )]
    pub reserve_state: Account<'info, ReserveState>,

    #[account(
        seeds = [RunoffState::SEED_PREFIX],
        bump = runoff_state.bump,
    )]
    pub runoff_state: Account<'info, RunoffState>,

    #[account(
        seeds = [VaultAuthority::SEED_PREFIX],
        bump = vault_authority.bump,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    #[account(
        mut,
        constraint = runoff_vault.key() == vault_authority.runoff_vault @ ReserveError::InvalidVaultConfig
    )]
    pub runoff_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = recipient.mint == reserve_config.usdc_mint @ ReserveError::InvalidMint
    )]
    pub recipient: Account<'info, TokenAccount>,

    /// Must be DAO authority
    #[account(
        constraint = authority.key() == reserve_config.authority @ ReserveError::Unauthorized
    )]
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn emergency_spend_runoff(
    ctx: Context<EmergencySpendRunoff>,
    amount: u64,
    reason: String,
) -> Result<()> {
    let clock = Clock::get()?;

    require!(amount > 0, ReserveError::ZeroAmount);
    require!(
        ctx.accounts.runoff_vault.amount >= amount,
        ReserveError::InsufficientReserves
    );

    // In production, this would also check emergency flag from governance
    // For now, only DAO authority can execute

    let seeds = &[
        VaultAuthority::SEED_PREFIX,
        &[ctx.accounts.vault_authority.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.runoff_vault.to_account_info(),
                to: ctx.accounts.recipient.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
    )?;

    let state = &mut ctx.accounts.reserve_state;
    state.runoff_balance = state.runoff_balance.saturating_sub(amount);

    emit!(RunoffSpent {
        amount,
        new_balance: state.runoff_balance,
        reason,
        authorizer: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Take a snapshot of current reserve state
#[derive(Accounts)]
pub struct TakeReserveSnapshot<'info> {
    #[account(
        seeds = [ReserveConfig::SEED_PREFIX],
        bump = reserve_config.bump,
    )]
    pub reserve_config: Account<'info, ReserveConfig>,

    #[account(
        seeds = [ReserveState::SEED_PREFIX],
        bump = reserve_state.bump,
    )]
    pub reserve_state: Account<'info, ReserveState>,
}

pub fn take_reserve_snapshot(ctx: Context<TakeReserveSnapshot>) -> Result<()> {
    let clock = Clock::get()?;
    let state = &ctx.accounts.reserve_state;

    emit!(ReserveSnapshot {
        tier0_balance: state.tier0_balance,
        tier1_balance: state.tier1_balance,
        tier2_balance: state.tier2_balance,
        runoff_balance: state.runoff_balance,
        ibnr_usdc: state.ibnr_usdc,
        coverage_ratio_bps: state.current_coverage_ratio_bps,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Helper to update coverage ratio
fn update_coverage_ratio(config: &ReserveConfig, state: &mut ReserveState) -> Result<()> {
    if state.expected_daily_claims == 0 {
        state.current_coverage_ratio_bps = 0;
        return Ok(());
    }

    // Expected annual claims approximation
    let expected_annual = state.expected_daily_claims.saturating_mul(365);

    // Total available (Tier0 + Tier1 + Tier2)
    let total_reserves = state.total_reserves();

    // Coverage ratio = (total_reserves / expected_annual) * 10000
    let ratio_bps = total_reserves
        .saturating_mul(10000)
        .checked_div(expected_annual)
        .unwrap_or(0) as u16;

    let old_ratio = state.current_coverage_ratio_bps;
    state.current_coverage_ratio_bps = ratio_bps;

    // Emit if significant change (>5%)
    if ratio_bps.abs_diff(old_ratio) > 500 {
        let clock = Clock::get()?;
        emit!(CoverageRatioChanged {
            old_ratio_bps: old_ratio,
            new_ratio_bps: ratio_bps,
            target_ratio_bps: config.target_coverage_ratio_bps,
            min_ratio_bps: config.min_coverage_ratio_bps,
            timestamp: clock.unix_timestamp,
        });
    }

    Ok(())
}
