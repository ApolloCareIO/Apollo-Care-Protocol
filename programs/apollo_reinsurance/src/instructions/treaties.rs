use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::ReinsuranceError;
use crate::events::*;
use crate::state::{ReinsuranceConfig, ReinsuranceLayerType, ReinsuranceTreaty, TreatyStatus};

// ============================================================================
// CREATE TREATY
// ============================================================================

#[derive(Accounts)]
#[instruction(params: CreateTreatyParams)]
pub struct CreateTreaty<'info> {
    #[account(
        mut,
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,

    #[account(
        init,
        payer = authority,
        space = ReinsuranceTreaty::SIZE,
        seeds = [
            b"treaty",
            config.key().as_ref(),
            &(config.total_treaties + 1).to_le_bytes()
        ],
        bump
    )]
    pub treaty: Account<'info, ReinsuranceTreaty>,

    /// Must be config authority or reinsurance committee
    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateTreatyParams {
    /// Type of reinsurance layer
    pub layer_type: ReinsuranceLayerType,

    /// Reinsurer identifier hash (off-chain reference)
    pub reinsurer_id: [u8; 32],

    /// Treaty effective date
    pub effective_date: i64,

    /// Treaty expiration date
    pub expiration_date: i64,

    /// Attachment point (USDC, 6 decimals)
    /// For specific: per-member threshold (e.g., $100,000)
    /// For aggregate: stored as USDC equivalent of expected claims at trigger %
    pub attachment_point: u64,

    /// Coinsurance rate Apollo retains (basis points)
    /// E.g., 2000 = 20% retained, 80% to reinsurer
    pub coinsurance_rate_bps: u16,

    /// Maximum coverage per occurrence (0 = unlimited)
    pub coverage_limit: u64,

    /// For aggregate layers: trigger ratio in bps
    pub trigger_ratio_bps: u16,

    /// For aggregate layers: ceiling ratio in bps
    pub ceiling_ratio_bps: u16,

    /// Annual premium for this treaty
    pub annual_premium: u64,

    /// Optional notes hash
    pub notes_hash: [u8; 32],
}

pub fn create_treaty(ctx: Context<CreateTreaty>, params: CreateTreatyParams) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let treaty = &mut ctx.accounts.treaty;
    let clock = Clock::get()?;

    // Validate authority
    let is_authorized = ctx.accounts.authority.key() == config.authority
        || ctx.accounts.authority.key() == config.reinsurance_committee;
    require!(is_authorized, ReinsuranceError::Unauthorized);

    // Validate dates
    require!(
        params.effective_date < params.expiration_date,
        ReinsuranceError::InvalidTreatyDates
    );

    // Validate attachment point
    require!(
        params.attachment_point > 0,
        ReinsuranceError::InvalidAttachmentPoint
    );

    // Validate coinsurance rate (0-100%)
    require!(
        params.coinsurance_rate_bps <= 10_000,
        ReinsuranceError::InvalidCoinsuranceRate
    );

    // For aggregate/catastrophic, validate trigger ratios
    if params.layer_type == ReinsuranceLayerType::AggregateStopLoss
        || params.layer_type == ReinsuranceLayerType::Catastrophic
    {
        require!(
            params.trigger_ratio_bps > 10_000,
            ReinsuranceError::InvalidTriggerRatio
        );
        require!(
            params.ceiling_ratio_bps > params.trigger_ratio_bps,
            ReinsuranceError::InvalidCeilingRatio
        );
    }

    // Increment treaty counter
    config.total_treaties = config
        .total_treaties
        .checked_add(1)
        .ok_or(ReinsuranceError::Overflow)?;

    // Initialize treaty
    treaty.treaty_id = config.total_treaties as u64;
    treaty.layer_type = params.layer_type;
    treaty.status = TreatyStatus::Pending;
    treaty.reinsurer_id = params.reinsurer_id;
    treaty.effective_date = params.effective_date;
    treaty.expiration_date = params.expiration_date;
    treaty.attachment_point = params.attachment_point;
    treaty.coinsurance_rate_bps = params.coinsurance_rate_bps;
    treaty.coverage_limit = params.coverage_limit;
    treaty.trigger_ratio_bps = params.trigger_ratio_bps;
    treaty.ceiling_ratio_bps = params.ceiling_ratio_bps;
    treaty.annual_premium = params.annual_premium;
    treaty.authority = ctx.accounts.authority.key();
    treaty.last_updated = clock.unix_timestamp;
    treaty.notes_hash = params.notes_hash;
    treaty.bump = ctx.bumps.treaty;

    // Initialize counters
    treaty.premium_paid = 0;
    treaty.total_claims_submitted = 0;
    treaty.total_recoveries_received = 0;
    treaty.recovery_claims_count = 0;
    treaty.claims_settled_count = 0;
    treaty.claims_pending_count = 0;

    emit!(TreatyCreated {
        treaty_id: treaty.treaty_id,
        treaty_pubkey: treaty.key(),
        layer_type: params.layer_type,
        reinsurer_id: params.reinsurer_id,
        attachment_point: params.attachment_point,
        coinsurance_rate_bps: params.coinsurance_rate_bps,
        coverage_limit: params.coverage_limit,
        annual_premium: params.annual_premium,
        effective_date: params.effective_date,
        expiration_date: params.expiration_date,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Treaty {} created: {:?}",
        treaty.treaty_id,
        params.layer_type
    );

    Ok(())
}

// ============================================================================
// ACTIVATE TREATY
// ============================================================================

#[derive(Accounts)]
pub struct ActivateTreaty<'info> {
    #[account(
        mut,
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,

    #[account(
        mut,
        constraint = treaty.status == TreatyStatus::Pending @ ReinsuranceError::TreatyCannotBeModified,
    )]
    pub treaty: Account<'info, ReinsuranceTreaty>,

    pub authority: Signer<'info>,
}

pub fn activate_treaty(ctx: Context<ActivateTreaty>) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let treaty = &mut ctx.accounts.treaty;
    let clock = Clock::get()?;

    // Validate authority
    let is_authorized = ctx.accounts.authority.key() == config.authority
        || ctx.accounts.authority.key() == config.reinsurance_committee
        || ctx.accounts.authority.key() == treaty.authority;
    require!(is_authorized, ReinsuranceError::Unauthorized);

    // Check premium is paid (at least partially)
    // For initial activation, we may require full premium or partial
    // Here we require at least 25% of annual premium
    let min_premium = treaty.annual_premium / 4;
    require!(
        treaty.premium_paid >= min_premium,
        ReinsuranceError::PremiumNotPaid
    );

    // Activate
    treaty.status = TreatyStatus::Active;
    treaty.last_updated = clock.unix_timestamp;

    config.active_treaties = config
        .active_treaties
        .checked_add(1)
        .ok_or(ReinsuranceError::Overflow)?;

    emit!(TreatyActivated {
        treaty_id: treaty.treaty_id,
        treaty_pubkey: treaty.key(),
        layer_type: treaty.layer_type,
        activated_by: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });

    msg!("Treaty {} activated", treaty.treaty_id);

    Ok(())
}

// ============================================================================
// PAY PREMIUM
// ============================================================================

#[derive(Accounts)]
pub struct PayPremium<'info> {
    #[account(
        mut,
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,

    #[account(
        mut,
        constraint = treaty.status != TreatyStatus::Cancelled @ ReinsuranceError::TreatyCannotBeModified,
    )]
    pub treaty: Account<'info, ReinsuranceTreaty>,

    /// Source of premium payment (protocol treasury)
    #[account(mut)]
    pub premium_source: Account<'info, TokenAccount>,

    /// Destination for premium (could be escrow or direct to reinsurer's agent)
    #[account(mut)]
    pub premium_destination: Account<'info, TokenAccount>,

    /// Authority to sign transfer
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn pay_premium(ctx: Context<PayPremium>, amount: u64) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let treaty = &mut ctx.accounts.treaty;
    let clock = Clock::get()?;

    // Validate amount
    require!(amount > 0, ReinsuranceError::ZeroAmount);

    // Check budget
    let new_ytd = config
        .premium_paid_ytd
        .checked_add(amount)
        .ok_or(ReinsuranceError::Overflow)?;
    // Warning but don't block if over budget
    if new_ytd > config.premium_budget {
        msg!("Warning: Premium payment exceeds annual budget");
    }

    // Transfer premium
    let cpi_accounts = Transfer {
        from: ctx.accounts.premium_source.to_account_info(),
        to: ctx.accounts.premium_destination.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    // Update tracking
    treaty.premium_paid = treaty
        .premium_paid
        .checked_add(amount)
        .ok_or(ReinsuranceError::Overflow)?;
    treaty.last_updated = clock.unix_timestamp;

    config.premium_paid_ytd = new_ytd;

    emit!(PremiumPaid {
        treaty_id: treaty.treaty_id,
        treaty_pubkey: treaty.key(),
        amount,
        total_paid: treaty.premium_paid,
        annual_premium: treaty.annual_premium,
        payer: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Premium paid: {} USDC, total: {} of {} USDC",
        amount / 1_000_000,
        treaty.premium_paid / 1_000_000,
        treaty.annual_premium / 1_000_000
    );

    Ok(())
}

// ============================================================================
// UPDATE TREATY STATUS
// ============================================================================

#[derive(Accounts)]
pub struct UpdateTreatyStatus<'info> {
    #[account(
        mut,
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,

    #[account(mut)]
    pub treaty: Account<'info, ReinsuranceTreaty>,

    pub authority: Signer<'info>,
}

pub fn update_treaty_status(
    ctx: Context<UpdateTreatyStatus>,
    new_status: TreatyStatus,
    reason_hash: [u8; 32],
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let treaty = &mut ctx.accounts.treaty;
    let clock = Clock::get()?;

    // Validate authority
    let is_authorized = ctx.accounts.authority.key() == config.authority
        || ctx.accounts.authority.key() == config.reinsurance_committee;
    require!(is_authorized, ReinsuranceError::Unauthorized);

    let old_status = treaty.status;

    // Validate state transitions
    match (old_status, new_status) {
        // Valid transitions
        (TreatyStatus::Pending, TreatyStatus::Active) => {}
        (TreatyStatus::Pending, TreatyStatus::Cancelled) => {}
        (TreatyStatus::Active, TreatyStatus::Suspended) => {}
        (TreatyStatus::Active, TreatyStatus::Expired) => {}
        (TreatyStatus::Active, TreatyStatus::Cancelled) => {}
        (TreatyStatus::Suspended, TreatyStatus::Active) => {}
        (TreatyStatus::Suspended, TreatyStatus::Cancelled) => {}
        (TreatyStatus::Suspended, TreatyStatus::Expired) => {}
        _ => return Err(ReinsuranceError::TreatyCannotBeModified.into()),
    }

    // Update active count
    if old_status == TreatyStatus::Active && new_status != TreatyStatus::Active {
        config.active_treaties = config.active_treaties.saturating_sub(1);
    } else if old_status != TreatyStatus::Active && new_status == TreatyStatus::Active {
        config.active_treaties = config
            .active_treaties
            .checked_add(1)
            .ok_or(ReinsuranceError::Overflow)?;
    }

    treaty.status = new_status;
    treaty.last_updated = clock.unix_timestamp;

    emit!(TreatyStatusChanged {
        treaty_id: treaty.treaty_id,
        treaty_pubkey: treaty.key(),
        old_status,
        new_status,
        changed_by: ctx.accounts.authority.key(),
        reason: reason_hash,
        timestamp: clock.unix_timestamp,
    });

    // Special handling for expiration
    if new_status == TreatyStatus::Expired {
        emit!(TreatyExpired {
            treaty_id: treaty.treaty_id,
            treaty_pubkey: treaty.key(),
            layer_type: treaty.layer_type,
            total_claims_submitted: treaty.total_claims_submitted,
            total_recoveries_received: treaty.total_recoveries_received,
            timestamp: clock.unix_timestamp,
        });
    }

    msg!(
        "Treaty {} status: {:?} -> {:?}",
        treaty.treaty_id,
        old_status,
        new_status
    );

    Ok(())
}

// ============================================================================
// CHECK AND EXPIRE TREATIES
// ============================================================================

#[derive(Accounts)]
pub struct CheckTreatyExpiration<'info> {
    #[account(
        mut,
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,

    #[account(
        mut,
        constraint = treaty.status == TreatyStatus::Active @ ReinsuranceError::TreatyNotActive,
    )]
    pub treaty: Account<'info, ReinsuranceTreaty>,
}

/// Permissionless - anyone can call to expire a treaty past its date
pub fn check_treaty_expiration(ctx: Context<CheckTreatyExpiration>) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let treaty = &mut ctx.accounts.treaty;
    let clock = Clock::get()?;

    if clock.unix_timestamp > treaty.expiration_date {
        config.active_treaties = config.active_treaties.saturating_sub(1);
        treaty.status = TreatyStatus::Expired;
        treaty.last_updated = clock.unix_timestamp;

        emit!(TreatyExpired {
            treaty_id: treaty.treaty_id,
            treaty_pubkey: treaty.key(),
            layer_type: treaty.layer_type,
            total_claims_submitted: treaty.total_claims_submitted,
            total_recoveries_received: treaty.total_recoveries_received,
            timestamp: clock.unix_timestamp,
        });

        msg!("Treaty {} expired", treaty.treaty_id);
    }

    Ok(())
}

// ============================================================================
// UPDATE TREATY PARAMETERS
// ============================================================================

#[derive(Accounts)]
pub struct UpdateTreatyParams<'info> {
    #[account(
        seeds = [b"reinsurance_config"],
        bump = config.bump,
    )]
    pub config: Account<'info, ReinsuranceConfig>,

    #[account(
        mut,
        constraint = treaty.status == TreatyStatus::Pending @ ReinsuranceError::TreatyCannotBeModified,
    )]
    pub treaty: Account<'info, ReinsuranceTreaty>,

    pub authority: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UpdateTreatyParamsInput {
    pub attachment_point: Option<u64>,
    pub coinsurance_rate_bps: Option<u16>,
    pub coverage_limit: Option<u64>,
    pub annual_premium: Option<u64>,
    pub effective_date: Option<i64>,
    pub expiration_date: Option<i64>,
}

pub fn update_treaty_params(
    ctx: Context<UpdateTreatyParams>,
    params: UpdateTreatyParamsInput,
) -> Result<()> {
    let config = &ctx.accounts.config;
    let treaty = &mut ctx.accounts.treaty;
    let clock = Clock::get()?;

    // Validate authority
    let is_authorized = ctx.accounts.authority.key() == config.authority
        || ctx.accounts.authority.key() == config.reinsurance_committee
        || ctx.accounts.authority.key() == treaty.authority;
    require!(is_authorized, ReinsuranceError::Unauthorized);

    // Update fields if provided
    if let Some(attachment) = params.attachment_point {
        require!(attachment > 0, ReinsuranceError::InvalidAttachmentPoint);
        treaty.attachment_point = attachment;
    }

    if let Some(coinsurance) = params.coinsurance_rate_bps {
        require!(
            coinsurance <= 10_000,
            ReinsuranceError::InvalidCoinsuranceRate
        );
        treaty.coinsurance_rate_bps = coinsurance;
    }

    if let Some(limit) = params.coverage_limit {
        treaty.coverage_limit = limit;
    }

    if let Some(premium) = params.annual_premium {
        treaty.annual_premium = premium;
    }

    if let Some(effective) = params.effective_date {
        treaty.effective_date = effective;
    }

    if let Some(expiration) = params.expiration_date {
        treaty.expiration_date = expiration;
    }

    // Validate dates if both exist
    require!(
        treaty.effective_date < treaty.expiration_date,
        ReinsuranceError::InvalidTreatyDates
    );

    treaty.last_updated = clock.unix_timestamp;

    msg!("Treaty {} parameters updated", treaty.treaty_id);

    Ok(())
}
