// programs/apollo_risk_engine/src/instructions/rating.rs

use crate::errors::RiskEngineError;
use crate::events::{ContributionQuoted, RatingTableUpdated};
use crate::state::{AgeBand, ContributionQuote, RatingTable, RegionFactor, RiskConfig};
use anchor_lang::prelude::*;

/// Set/update the rating table
#[derive(Accounts)]
pub struct SetRatingTable<'info> {
    #[account(
        seeds = [RiskConfig::SEED_PREFIX],
        bump = risk_config.bump,
        constraint = risk_config.is_active @ RiskEngineError::RiskEngineNotActive
    )]
    pub risk_config: Account<'info, RiskConfig>,

    #[account(
        mut,
        seeds = [RatingTable::SEED_PREFIX],
        bump = rating_table.bump,
    )]
    pub rating_table: Account<'info, RatingTable>,

    /// Must be authorized (Actuarial Committee)
    #[account(
        constraint = authority.key() == risk_config.authority @ RiskEngineError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SetRatingTableParams {
    pub age_bands: Option<Vec<AgeBand>>,
    pub region_factors: Option<Vec<RegionFactor>>,
}

pub fn set_rating_table(ctx: Context<SetRatingTable>, params: SetRatingTableParams) -> Result<()> {
    let clock = Clock::get()?;
    let rating_table = &mut ctx.accounts.rating_table;

    if let Some(bands) = params.age_bands {
        // Validate age bands
        require!(bands.len() > 0, RiskEngineError::EmptyRatingTable);
        require!(bands.len() <= 10, RiskEngineError::TooManyAgeBands);

        // Check for overlaps and CMS 3:1 compliance
        let mut min_factor: u16 = u16::MAX;
        let mut max_factor: u16 = 0;

        for (i, band) in bands.iter().enumerate() {
            require!(
                band.min_age <= band.max_age,
                RiskEngineError::InvalidAgeBand
            );
            require!(band.factor_bps > 0, RiskEngineError::InvalidAgeBand);

            min_factor = min_factor.min(band.factor_bps);
            max_factor = max_factor.max(band.factor_bps);

            // Check for overlaps with other bands
            for (j, other) in bands.iter().enumerate() {
                if i != j {
                    let overlaps = !(band.max_age < other.min_age || band.min_age > other.max_age);
                    require!(!overlaps, RiskEngineError::OverlappingAgeBands);
                }
            }
        }

        // CMS 3:1 ratio check
        if min_factor > 0 {
            let ratio = (max_factor as u32 * 10000) / (min_factor as u32);
            require!(ratio <= 30000, RiskEngineError::AgeBandRatioExceeded); // 3.0x max
        }

        rating_table.age_bands = bands;
        rating_table.band_count = rating_table.age_bands.len() as u8;
    }

    if let Some(regions) = params.region_factors {
        require!(regions.len() <= 20, RiskEngineError::TooManyRegions);

        for region in &regions {
            require!(region.factor_bps > 0, RiskEngineError::InvalidRegionFactor);
        }

        rating_table.region_factors = regions;
    }

    rating_table.last_updated = clock.unix_timestamp;
    rating_table.last_updater = ctx.accounts.authority.key();

    emit!(RatingTableUpdated {
        updater: ctx.accounts.authority.key(),
        band_count: rating_table.band_count,
        region_count: rating_table.region_factors.len() as u8,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Quote a contribution for a member
#[derive(Accounts)]
pub struct QuoteContribution<'info> {
    #[account(
        seeds = [RiskConfig::SEED_PREFIX],
        bump = risk_config.bump,
        constraint = risk_config.is_active @ RiskEngineError::RiskEngineNotActive
    )]
    pub risk_config: Account<'info, RiskConfig>,

    #[account(
        seeds = [RatingTable::SEED_PREFIX],
        bump = rating_table.bump,
    )]
    pub rating_table: Account<'info, RatingTable>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct QuoteContributionParams {
    /// Primary member age
    pub age: u8,
    /// Is tobacco user
    pub is_tobacco_user: bool,
    /// Region code
    pub region_code: u8,
    /// Number of children (capped at max_children)
    pub num_children: u8,
    /// Number of additional adults
    pub num_additional_adults: u8,
    /// Ages of additional adults (for accurate pricing)
    pub additional_adult_ages: Vec<u8>,
}

pub fn quote_contribution(
    ctx: Context<QuoteContribution>,
    params: QuoteContributionParams,
) -> Result<ContributionQuote> {
    let clock = Clock::get()?;
    let config = &ctx.accounts.risk_config;
    let table = &ctx.accounts.rating_table;

    require!(
        params.age > 0 && params.age <= 64,
        RiskEngineError::InvalidAge
    );

    // Get factors
    let age_factor_bps = table.get_age_factor(params.age);
    let region_factor_bps = table.get_region_factor(params.region_code);
    let tobacco_factor_bps = if params.is_tobacco_user {
        config.tobacco_factor_bps
    } else {
        10000 // 1.0x
    };

    // Calculate base for primary adult
    // Base = base_rate * (age_factor / 10000) * (region_factor / 10000) * (tobacco_factor / 10000)
    let mut base_amount = config.base_rate_adult;
    base_amount = base_amount
        .saturating_mul(age_factor_bps as u64)
        .checked_div(10000)
        .ok_or(RiskEngineError::MathOverflow)?;

    // Add children (capped)
    let children_count = params.num_children.min(config.max_children);
    if children_count > 0 {
        let child_cost = config
            .base_rate_adult
            .saturating_mul(config.child_factor_bps as u64)
            .checked_div(10000)
            .ok_or(RiskEngineError::MathOverflow)?;
        base_amount = base_amount.saturating_add(child_cost.saturating_mul(children_count as u64));
    }

    // Add additional adults
    for adult_age in params
        .additional_adult_ages
        .iter()
        .take(params.num_additional_adults as usize)
    {
        let adult_age_factor = table.get_age_factor(*adult_age);
        let adult_cost = config
            .base_rate_adult
            .saturating_mul(adult_age_factor as u64)
            .checked_div(10000)
            .ok_or(RiskEngineError::MathOverflow)?;
        base_amount = base_amount.saturating_add(adult_cost);
    }

    // Apply region factor
    base_amount = base_amount
        .saturating_mul(region_factor_bps as u64)
        .checked_div(10000)
        .ok_or(RiskEngineError::MathOverflow)?;

    // Apply tobacco factor
    base_amount = base_amount
        .saturating_mul(tobacco_factor_bps as u64)
        .checked_div(10000)
        .ok_or(RiskEngineError::MathOverflow)?;

    // Apply ShockFactor
    let final_contribution = base_amount
        .saturating_mul(config.shock_factor_bps as u64)
        .checked_div(10000)
        .ok_or(RiskEngineError::MathOverflow)?;

    // Ensure minimum
    let final_contribution = final_contribution.max(config.min_contribution);

    let quote = ContributionQuote {
        base_amount,
        age_factor_bps,
        region_factor_bps,
        tobacco_factor_bps,
        shock_factor_bps: config.shock_factor_bps,
        final_contribution,
        quoted_at: clock.unix_timestamp,
    };

    emit!(ContributionQuoted {
        member: Pubkey::default(), // Would be set by caller
        age: params.age,
        is_tobacco: params.is_tobacco_user,
        region_code: params.region_code,
        base_amount,
        final_contribution,
        timestamp: clock.unix_timestamp,
    });

    Ok(quote)
}

/// Update base rate
#[derive(Accounts)]
pub struct UpdateBaseRate<'info> {
    #[account(
        mut,
        seeds = [RiskConfig::SEED_PREFIX],
        bump = risk_config.bump,
    )]
    pub risk_config: Account<'info, RiskConfig>,

    #[account(
        constraint = authority.key() == risk_config.authority @ RiskEngineError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn update_base_rate(ctx: Context<UpdateBaseRate>, new_base_rate: u64) -> Result<()> {
    require!(new_base_rate > 0, RiskEngineError::InvalidBasisPoints);
    ctx.accounts.risk_config.base_rate_adult = new_base_rate;
    Ok(())
}

/// Update tobacco factor
pub fn update_tobacco_factor(ctx: Context<UpdateBaseRate>, new_factor_bps: u16) -> Result<()> {
    require!(
        new_factor_bps >= 10000 && new_factor_bps <= 15000, // 1.0x to 1.5x
        RiskEngineError::InvalidTobaccoFactor
    );
    ctx.accounts.risk_config.tobacco_factor_bps = new_factor_bps;
    Ok(())
}

/// Update child factor
pub fn update_child_factor(ctx: Context<UpdateBaseRate>, new_factor_bps: u16) -> Result<()> {
    require!(
        new_factor_bps > 0 && new_factor_bps <= 10000, // 0% to 100%
        RiskEngineError::InvalidBasisPoints
    );
    ctx.accounts.risk_config.child_factor_bps = new_factor_bps;
    Ok(())
}
