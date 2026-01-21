// programs/apollo_governance/src/instructions/initialize.rs

use anchor_lang::prelude::*;
use crate::state::DaoConfig;
use crate::events::DaoInitialized;

#[derive(Accounts)]
pub struct InitializeDao<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + DaoConfig::INIT_SPACE,
        seeds = [DaoConfig::SEED_PREFIX],
        bump
    )]
    pub dao_config: Account<'info, DaoConfig>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeDaoParams {
    /// Initial Risk Committee multisig (can be updated later)
    pub risk_committee: Pubkey,
    /// Initial Actuarial Committee multisig
    pub actuarial_committee: Pubkey,
    /// Initial Claims Committee multisig
    pub claims_committee: Pubkey,
    /// Initial Treasury Committee multisig
    pub treasury_committee: Pubkey,
    /// Maximum emergency duration in seconds
    pub max_emergency_duration: i64,
}

pub fn handler(ctx: Context<InitializeDao>, params: InitializeDaoParams) -> Result<()> {
    let dao_config = &mut ctx.accounts.dao_config;
    let clock = Clock::get()?;

    dao_config.authority = ctx.accounts.authority.key();
    dao_config.risk_committee = params.risk_committee;
    dao_config.actuarial_committee = params.actuarial_committee;
    dao_config.claims_committee = params.claims_committee;
    dao_config.treasury_committee = params.treasury_committee;
    dao_config.emergency_active = false;
    dao_config.emergency_activated_at = 0;
    dao_config.max_emergency_duration = params.max_emergency_duration;
    dao_config.proposal_count = 0;
    dao_config.protocol_paused = false;
    dao_config.bump = ctx.bumps.dao_config;
    dao_config.reserved = vec![];

    emit!(DaoInitialized {
        authority: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
