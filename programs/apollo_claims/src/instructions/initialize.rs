// programs/apollo_claims/src/instructions/initialize.rs

use anchor_lang::prelude::*;
use crate::state::{ClaimsConfig, BenefitSchedule, AttestorRegistry, CategoryLimit};
use crate::errors::ClaimsError;
use crate::events::{ClaimsConfigInitialized, BenefitScheduleUpdated};

#[derive(Accounts)]
pub struct InitializeClaimsConfig<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + ClaimsConfig::INIT_SPACE,
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump
    )]
    pub claims_config: Account<'info, ClaimsConfig>,

    #[account(
        init,
        payer = authority,
        space = 8 + AttestorRegistry::INIT_SPACE,
        seeds = [AttestorRegistry::SEED_PREFIX],
        bump
    )]
    pub attestor_registry: Account<'info, AttestorRegistry>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeClaimsParams {
    pub governance_program: Pubkey,
    pub reserves_program: Pubkey,
    pub claims_committee: Pubkey,
    pub auto_approve_threshold: Option<u64>,
    pub shock_claim_threshold: Option<u64>,
    pub required_attestations: Option<u8>,
}

pub fn handler(ctx: Context<InitializeClaimsConfig>, params: InitializeClaimsParams) -> Result<()> {
    let clock = Clock::get()?;

    let config = &mut ctx.accounts.claims_config;
    config.authority = ctx.accounts.authority.key();
    config.governance_program = params.governance_program;
    config.reserves_program = params.reserves_program;
    config.claims_committee = params.claims_committee;
    config.total_claims_submitted = 0;
    config.total_claims_approved = 0;
    config.total_claims_denied = 0;
    config.total_paid_out = 0;
    config.auto_approve_threshold = params.auto_approve_threshold
        .unwrap_or(ClaimsConfig::DEFAULT_AUTO_APPROVE);
    config.shock_claim_threshold = params.shock_claim_threshold
        .unwrap_or(ClaimsConfig::DEFAULT_SHOCK_THRESHOLD);
    config.required_attestations = params.required_attestations
        .unwrap_or(ClaimsConfig::DEFAULT_REQUIRED_ATTESTATIONS);
    config.max_attestation_time = ClaimsConfig::DEFAULT_MAX_ATTESTATION_TIME;
    config.is_active = true;
    config.bump = ctx.bumps.claims_config;

    let registry = &mut ctx.accounts.attestor_registry;
    registry.attestors = vec![];
    registry.attestor_count = 0;
    registry.total_attestations = 0;
    registry.bump = ctx.bumps.attestor_registry;

    emit!(ClaimsConfigInitialized {
        authority: ctx.accounts.authority.key(),
        claims_committee: params.claims_committee,
        auto_approve_threshold: config.auto_approve_threshold,
        shock_claim_threshold: config.shock_claim_threshold,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Set/update benefit schedule
#[derive(Accounts)]
#[instruction(name: String)]
pub struct SetBenefitSchedule<'info> {
    #[account(
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump = claims_config.bump,
    )]
    pub claims_config: Account<'info, ClaimsConfig>,

    #[account(
        init_if_needed,
        payer = authority,
        space = 8 + BenefitSchedule::INIT_SPACE,
        seeds = [BenefitSchedule::SEED_PREFIX, name.as_bytes()],
        bump
    )]
    pub benefit_schedule: Account<'info, BenefitSchedule>,

    #[account(
        mut,
        constraint = authority.key() == claims_config.authority @ ClaimsError::Unauthorized
    )]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SetBenefitScheduleParams {
    pub name: String,
    pub individual_annual_max: u64,
    pub family_annual_max: u64,
    pub per_incident_max: u64,
    pub individual_deductible: u64,
    pub family_deductible: u64,
    pub coinsurance_bps: u16,
    pub oop_max_individual: u64,
    pub oop_max_family: u64,
    pub preexisting_waiting_days: u16,
    pub category_limits: Option<Vec<CategoryLimit>>,
}

pub fn set_benefit_schedule(
    ctx: Context<SetBenefitSchedule>,
    params: SetBenefitScheduleParams,
) -> Result<()> {
    let clock = Clock::get()?;

    require!(params.coinsurance_bps <= 10000, ClaimsError::InvalidBenefitSchedule);
    require!(params.individual_annual_max > 0, ClaimsError::InvalidBenefitSchedule);
    require!(params.family_annual_max >= params.individual_annual_max, ClaimsError::InvalidBenefitSchedule);

    let schedule = &mut ctx.accounts.benefit_schedule;
    schedule.name = params.name.clone();
    schedule.individual_annual_max = params.individual_annual_max;
    schedule.family_annual_max = params.family_annual_max;
    schedule.per_incident_max = params.per_incident_max;
    schedule.individual_deductible = params.individual_deductible;
    schedule.family_deductible = params.family_deductible;
    schedule.coinsurance_bps = params.coinsurance_bps;
    schedule.oop_max_individual = params.oop_max_individual;
    schedule.oop_max_family = params.oop_max_family;
    schedule.preexisting_waiting_days = params.preexisting_waiting_days;
    schedule.is_active = true;
    schedule.category_limits = params.category_limits.unwrap_or_default();
    schedule.last_updated = clock.unix_timestamp;
    schedule.bump = ctx.bumps.benefit_schedule;

    emit!(BenefitScheduleUpdated {
        name: params.name,
        individual_annual_max: params.individual_annual_max,
        family_annual_max: params.family_annual_max,
        updater: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Add an attestor to the registry
#[derive(Accounts)]
pub struct ManageAttestor<'info> {
    #[account(
        seeds = [ClaimsConfig::SEED_PREFIX],
        bump = claims_config.bump,
    )]
    pub claims_config: Account<'info, ClaimsConfig>,

    #[account(
        mut,
        seeds = [AttestorRegistry::SEED_PREFIX],
        bump = attestor_registry.bump,
    )]
    pub attestor_registry: Account<'info, AttestorRegistry>,

    #[account(
        constraint = authority.key() == claims_config.authority @ ClaimsError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn add_attestor(ctx: Context<ManageAttestor>, attestor: Pubkey) -> Result<()> {
    let registry = &mut ctx.accounts.attestor_registry;
    let clock = Clock::get()?;

    require!(
        registry.attestors.len() < AttestorRegistry::MAX_ATTESTORS,
        ClaimsError::MaxAttestorsReached
    );
    require!(
        !registry.is_attestor(&attestor),
        ClaimsError::AlreadyAttested
    );

    registry.attestors.push(attestor);
    registry.attestor_count += 1;

    emit!(crate::events::AttestorAdded {
        attestor,
        total_attestors: registry.attestor_count,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

pub fn remove_attestor(ctx: Context<ManageAttestor>, attestor: Pubkey) -> Result<()> {
    let registry = &mut ctx.accounts.attestor_registry;
    let clock = Clock::get()?;

    require!(
        registry.is_attestor(&attestor),
        ClaimsError::AttestorNotRegistered
    );

    registry.attestors.retain(|a| a != &attestor);
    registry.attestor_count -= 1;

    emit!(crate::events::AttestorRemoved {
        attestor,
        total_attestors: registry.attestor_count,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
