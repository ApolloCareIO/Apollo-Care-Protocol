// programs/apollo_reserves/src/instructions/routing.rs

use crate::errors::ReserveError;
use crate::events::ContributionRouted;
use crate::state::{ContributionRouting, ReserveConfig, ReserveState, VaultAuthority};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

/// Route a contribution to appropriate vaults based on reserve policy
#[derive(Accounts)]
pub struct RouteContribution<'info> {
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

    /// Source token account (member's contribution)
    #[account(
        mut,
        constraint = source.mint == reserve_config.usdc_mint @ ReserveError::InvalidMint
    )]
    pub source: Account<'info, TokenAccount>,

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

    #[account(
        mut,
        constraint = admin_vault.key() == vault_authority.admin_vault @ ReserveError::InvalidVaultConfig
    )]
    pub admin_vault: Account<'info, TokenAccount>,

    /// Member/contributor signing the transfer
    pub contributor: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn route_contribution(ctx: Context<RouteContribution>, total_amount: u64) -> Result<()> {
    require!(total_amount > 0, ReserveError::ZeroAmount);

    let clock = Clock::get()?;
    let config = &ctx.accounts.reserve_config;
    let state = &mut ctx.accounts.reserve_state;

    // Calculate routing based on policy
    let routing = calculate_routing(config, state, total_amount)?;

    // Verify total matches
    let sum = routing
        .to_tier0
        .saturating_add(routing.to_tier1)
        .saturating_add(routing.to_tier2)
        .saturating_add(routing.to_admin);
    require!(sum == total_amount, ReserveError::RoutingMismatch);

    // Execute transfers
    // To Tier 0 (liquidity buffer - majority goes here initially)
    if routing.to_tier0 > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.source.to_account_info(),
                    to: ctx.accounts.tier0_vault.to_account_info(),
                    authority: ctx.accounts.contributor.to_account_info(),
                },
            ),
            routing.to_tier0,
        )?;
        state.tier0_balance = state.tier0_balance.saturating_add(routing.to_tier0);
    }

    // To Tier 1 (reserve margin)
    if routing.to_tier1 > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.source.to_account_info(),
                    to: ctx.accounts.tier1_vault.to_account_info(),
                    authority: ctx.accounts.contributor.to_account_info(),
                },
            ),
            routing.to_tier1,
        )?;
        state.tier1_balance = state.tier1_balance.saturating_add(routing.to_tier1);
    }

    // To Tier 2 (treasury)
    if routing.to_tier2 > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.source.to_account_info(),
                    to: ctx.accounts.tier2_vault.to_account_info(),
                    authority: ctx.accounts.contributor.to_account_info(),
                },
            ),
            routing.to_tier2,
        )?;
        state.tier2_balance = state.tier2_balance.saturating_add(routing.to_tier2);
    }

    // To Admin (operations)
    if routing.to_admin > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.source.to_account_info(),
                    to: ctx.accounts.admin_vault.to_account_info(),
                    authority: ctx.accounts.contributor.to_account_info(),
                },
            ),
            routing.to_admin,
        )?;
    }

    // Update total contributions
    state.total_contributions_received = state
        .total_contributions_received
        .saturating_add(total_amount);

    emit!(ContributionRouted {
        member: ctx.accounts.contributor.key(),
        total_amount,
        to_tier0: routing.to_tier0,
        to_tier1: routing.to_tier1,
        to_tier2: routing.to_tier2,
        to_admin: routing.to_admin,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Calculate how to route a contribution based on current reserve levels
fn calculate_routing(
    config: &ReserveConfig,
    state: &ReserveState,
    total: u64,
) -> Result<ContributionRouting> {
    // Calculate targets
    let tier0_target = state
        .expected_daily_claims
        .saturating_mul(config.tier0_target_days as u64);
    let tier1_target = state
        .expected_daily_claims
        .saturating_mul(config.tier1_target_days as u64)
        .saturating_add(state.ibnr_usdc);

    // Admin load (always taken)
    let admin_amount = total
        .saturating_mul(config.admin_load_bps as u64)
        .checked_div(10000)
        .unwrap_or(0);

    // Reserve margin (goes to Tier 1)
    let reserve_margin = total
        .saturating_mul(config.reserve_margin_bps as u64)
        .checked_div(10000)
        .unwrap_or(0);

    let remaining_after_loads = total
        .saturating_sub(admin_amount)
        .saturating_sub(reserve_margin);

    // Priority: Fill Tier0 first, then Tier1, overflow to Tier2
    let tier0_deficit = tier0_target.saturating_sub(state.tier0_balance);
    let tier1_deficit = tier1_target.saturating_sub(state.tier1_balance);

    let (to_tier0, leftover_after_tier0) = if tier0_deficit > 0 {
        let fill = remaining_after_loads.min(tier0_deficit);
        (fill, remaining_after_loads.saturating_sub(fill))
    } else {
        (0, remaining_after_loads)
    };

    let to_tier1 = reserve_margin.saturating_add(if tier1_deficit > 0 {
        leftover_after_tier0.min(tier1_deficit)
    } else {
        0
    });

    let to_tier2 = total
        .saturating_sub(to_tier0)
        .saturating_sub(to_tier1)
        .saturating_sub(admin_amount);

    Ok(ContributionRouting {
        to_tier0,
        to_tier1,
        to_tier2,
        to_admin: admin_amount,
        total,
    })
}

/// Direct deposit to a specific tier (for treasury operations)
#[derive(Accounts)]
pub struct DepositToTier<'info> {
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
        seeds = [VaultAuthority::SEED_PREFIX],
        bump = vault_authority.bump,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    #[account(
        mut,
        constraint = source.mint == reserve_config.usdc_mint @ ReserveError::InvalidMint
    )]
    pub source: Account<'info, TokenAccount>,

    /// The target vault (tier0, tier1, or tier2)
    #[account(mut)]
    pub target_vault: Account<'info, TokenAccount>,

    pub depositor: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum TierTarget {
    Tier0,
    Tier1,
    Tier2,
}

pub fn deposit_to_tier(ctx: Context<DepositToTier>, tier: TierTarget, amount: u64) -> Result<()> {
    require!(amount > 0, ReserveError::ZeroAmount);

    let vault_authority = &ctx.accounts.vault_authority;

    // Verify target vault matches requested tier
    let expected_vault = match tier {
        TierTarget::Tier0 => vault_authority.tier0_vault,
        TierTarget::Tier1 => vault_authority.tier1_vault,
        TierTarget::Tier2 => vault_authority.tier2_vault,
    };
    require!(
        ctx.accounts.target_vault.key() == expected_vault,
        ReserveError::InvalidVaultConfig
    );

    // Execute transfer
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.source.to_account_info(),
                to: ctx.accounts.target_vault.to_account_info(),
                authority: ctx.accounts.depositor.to_account_info(),
            },
        ),
        amount,
    )?;

    // Update state
    let state = &mut ctx.accounts.reserve_state;
    match tier {
        TierTarget::Tier0 => {
            state.tier0_balance = state.tier0_balance.saturating_add(amount);
        }
        TierTarget::Tier1 => {
            state.tier1_balance = state.tier1_balance.saturating_add(amount);
        }
        TierTarget::Tier2 => {
            state.tier2_balance = state.tier2_balance.saturating_add(amount);
        }
    }

    Ok(())
}
