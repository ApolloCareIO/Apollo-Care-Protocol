// programs/apollo_reserves/src/instructions/vaults.rs

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint};
use crate::state::{ReserveConfig, VaultAuthority};
use crate::errors::ReserveError;
use crate::events::VaultsCreated;

/// Create all reserve vaults
#[derive(Accounts)]
pub struct CreateVaults<'info> {
    #[account(
        seeds = [ReserveConfig::SEED_PREFIX],
        bump = reserve_config.bump,
        constraint = reserve_config.is_initialized @ ReserveError::NotInitialized
    )]
    pub reserve_config: Box<Account<'info, ReserveConfig>>,

    #[account(
        init,
        payer = authority,
        space = 8 + VaultAuthority::INIT_SPACE,
        seeds = [VaultAuthority::SEED_PREFIX],
        bump
    )]
    pub vault_authority: Box<Account<'info, VaultAuthority>>,

    /// Tier 0 vault - liquidity buffer
    #[account(
        init,
        payer = authority,
        token::mint = usdc_mint,
        token::authority = vault_authority,
        seeds = [b"tier0_vault"],
        bump
    )]
    pub tier0_vault: Box<Account<'info, TokenAccount>>,

    /// Tier 1 vault - operating reserve
    #[account(
        init,
        payer = authority,
        token::mint = usdc_mint,
        token::authority = vault_authority,
        seeds = [b"tier1_vault"],
        bump
    )]
    pub tier1_vault: Box<Account<'info, TokenAccount>>,

    /// Tier 2 vault - contingent capital
    #[account(
        init,
        payer = authority,
        token::mint = usdc_mint,
        token::authority = vault_authority,
        seeds = [b"tier2_vault"],
        bump
    )]
    pub tier2_vault: Box<Account<'info, TokenAccount>>,

    /// Run-off reserve vault - segregated emergency fund
    #[account(
        init,
        payer = authority,
        token::mint = usdc_mint,
        token::authority = vault_authority,
        seeds = [b"runoff_vault"],
        bump
    )]
    pub runoff_vault: Box<Account<'info, TokenAccount>>,

    /// Admin/operations vault
    #[account(
        init,
        payer = authority,
        token::mint = usdc_mint,
        token::authority = vault_authority,
        seeds = [b"admin_vault"],
        bump
    )]
    pub admin_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        constraint = usdc_mint.key() == reserve_config.usdc_mint @ ReserveError::InvalidMint
    )]
    pub usdc_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = authority.key() == reserve_config.authority @ ReserveError::Unauthorized
    )]
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_vaults(ctx: Context<CreateVaults>) -> Result<()> {
    let clock = Clock::get()?;

    let vault_authority = &mut ctx.accounts.vault_authority;
    vault_authority.tier0_vault = ctx.accounts.tier0_vault.key();
    vault_authority.tier1_vault = ctx.accounts.tier1_vault.key();
    vault_authority.tier2_vault = ctx.accounts.tier2_vault.key();
    vault_authority.runoff_vault = ctx.accounts.runoff_vault.key();
    vault_authority.admin_vault = ctx.accounts.admin_vault.key();
    vault_authority.usdc_mint = ctx.accounts.usdc_mint.key();
    vault_authority.bump = ctx.bumps.vault_authority;

    emit!(VaultsCreated {
        vault_authority: ctx.accounts.vault_authority.key(),
        tier0_vault: ctx.accounts.tier0_vault.key(),
        tier1_vault: ctx.accounts.tier1_vault.key(),
        tier2_vault: ctx.accounts.tier2_vault.key(),
        runoff_vault: ctx.accounts.runoff_vault.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Refill Tier 0 from Tier 1 if needed
#[derive(Accounts)]
pub struct RefillTier0<'info> {
    #[account(
        seeds = [ReserveConfig::SEED_PREFIX],
        bump = reserve_config.bump,
    )]
    pub reserve_config: Account<'info, ReserveConfig>,

    #[account(
        mut,
        seeds = [crate::state::ReserveState::SEED_PREFIX],
        bump = reserve_state.bump,
    )]
    pub reserve_state: Account<'info, crate::state::ReserveState>,

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

    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn refill_tier0(ctx: Context<RefillTier0>, amount: u64) -> Result<()> {
    require!(amount > 0, ReserveError::ZeroAmount);
    require!(
        ctx.accounts.tier1_vault.amount >= amount,
        ReserveError::InsufficientTier1
    );

    let clock = Clock::get()?;

    // Transfer from Tier1 to Tier0
    let seeds = &[
        VaultAuthority::SEED_PREFIX,
        &[ctx.accounts.vault_authority.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.tier1_vault.to_account_info(),
                to: ctx.accounts.tier0_vault.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
    )?;

    // Update state
    let state = &mut ctx.accounts.reserve_state;
    state.tier0_balance = state.tier0_balance.saturating_add(amount);
    state.tier1_balance = state.tier1_balance.saturating_sub(amount);

    emit!(crate::events::TierRefilled {
        from_tier: crate::state::WaterfallSource::Tier1,
        to_tier: crate::state::WaterfallSource::Tier0,
        amount,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Refill Tier 1 from Tier 2 if needed
#[derive(Accounts)]
pub struct RefillTier1<'info> {
    #[account(
        seeds = [ReserveConfig::SEED_PREFIX],
        bump = reserve_config.bump,
    )]
    pub reserve_config: Account<'info, ReserveConfig>,

    #[account(
        mut,
        seeds = [crate::state::ReserveState::SEED_PREFIX],
        bump = reserve_state.bump,
    )]
    pub reserve_state: Account<'info, crate::state::ReserveState>,

    #[account(
        seeds = [VaultAuthority::SEED_PREFIX],
        bump = vault_authority.bump,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

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

    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn refill_tier1(ctx: Context<RefillTier1>, amount: u64) -> Result<()> {
    require!(amount > 0, ReserveError::ZeroAmount);
    require!(
        ctx.accounts.tier2_vault.amount >= amount,
        ReserveError::InsufficientTier2
    );

    let clock = Clock::get()?;

    let seeds = &[
        VaultAuthority::SEED_PREFIX,
        &[ctx.accounts.vault_authority.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.tier2_vault.to_account_info(),
                to: ctx.accounts.tier1_vault.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
    )?;

    let state = &mut ctx.accounts.reserve_state;
    state.tier1_balance = state.tier1_balance.saturating_add(amount);
    state.tier2_balance = state.tier2_balance.saturating_sub(amount);

    emit!(crate::events::TierRefilled {
        from_tier: crate::state::WaterfallSource::Tier2,
        to_tier: crate::state::WaterfallSource::Tier1,
        amount,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
