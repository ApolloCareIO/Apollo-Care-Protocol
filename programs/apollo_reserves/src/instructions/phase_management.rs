// programs/apollo_reserves/src/instructions/phase_management.rs
//
// Phase Transition Management
// ===========================
// Manages the protocol's evolution through regulatory phases:
// Phase 1: Cost-Sharing Ministry (HCSM)
// Phase 2: Hybrid/Regulatory Sandbox
// Phase 3: Fully Licensed Insurer

use crate::errors::ReservesError;
use crate::state::{
    CohortMetrics, Phase1Requirements, Phase2Requirements, PhaseManager, ProtocolPhase,
    ReinsuranceConfig, ReserveState,
};
use anchor_lang::prelude::*;

// =============================================================================
// INITIALIZE PHASE MANAGER
// =============================================================================

/// Initialize the phase manager (start at Phase 1)
#[derive(Accounts)]
pub struct InitializePhaseManager<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + PhaseManager::INIT_SPACE,
        seeds = [PhaseManager::SEED_PREFIX],
        bump
    )]
    pub phase_manager: Account<'info, PhaseManager>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_phase_manager(ctx: Context<InitializePhaseManager>) -> Result<()> {
    let clock = Clock::get()?;
    let manager = &mut ctx.accounts.phase_manager;

    manager.authority = ctx.accounts.authority.key();
    manager.current_phase = ProtocolPhase::Phase1Hcsm;
    manager.phase1_start = clock.unix_timestamp;
    manager.phase2_start = 0;
    manager.phase3_start = 0;
    manager.phase1_requirements = Phase1Requirements::default();
    manager.phase2_requirements = Phase2Requirements::default();
    manager.transition_pending = false;
    manager.pending_target_phase = ProtocolPhase::Phase1Hcsm;
    manager.bump = ctx.bumps.phase_manager;

    emit!(PhaseManagerInitialized {
        authority: ctx.accounts.authority.key(),
        initial_phase: ProtocolPhase::Phase1Hcsm,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

// =============================================================================
// CHECK PHASE 1 → 2 ELIGIBILITY
// =============================================================================

/// Check if protocol meets Phase 1 → 2 transition requirements
#[derive(Accounts)]
pub struct CheckPhase1Eligibility<'info> {
    #[account(
        seeds = [PhaseManager::SEED_PREFIX],
        bump = phase_manager.bump,
        constraint = phase_manager.current_phase == ProtocolPhase::Phase1Hcsm
            @ ReservesError::InvalidPhaseTransition
    )]
    pub phase_manager: Account<'info, PhaseManager>,

    #[account(
        seeds = [ReserveState::SEED_PREFIX],
        bump,
    )]
    pub reserve_state: Account<'info, ReserveState>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Phase1EligibilityStatus {
    pub eligible: bool,
    pub months_operating: u8,
    pub months_required: u8,
    pub current_car_bps: u16,
    pub required_car_bps: u16,
    pub audit_completed: bool,
    pub financial_audit_completed: bool,
    pub missing_requirements: Vec<String>,
}

pub fn check_phase1_eligibility(
    ctx: Context<CheckPhase1Eligibility>,
    current_members: u32,
    current_loss_ratio_bps: u16,
    consecutive_good_months: u8,
    current_car_bps: u16,
) -> Result<Phase1EligibilityStatus> {
    let clock = Clock::get()?;
    let manager = &ctx.accounts.phase_manager;
    let reqs = &manager.phase1_requirements;

    let months_operating = ((clock.unix_timestamp - manager.phase1_start) / (30 * 86400)) as u8;

    let mut missing = Vec::new();

    // Check each requirement
    if months_operating < reqs.min_months_operation {
        missing.push(format!(
            "Need {} months operation, have {}",
            reqs.min_months_operation, months_operating
        ));
    }

    if current_members < reqs.min_members {
        missing.push(format!(
            "Need {} members, have {}",
            reqs.min_members, current_members
        ));
    }

    if current_loss_ratio_bps < reqs.min_loss_ratio_bps {
        missing.push(format!(
            "Loss ratio {}bps below minimum {}bps",
            current_loss_ratio_bps, reqs.min_loss_ratio_bps
        ));
    }

    if current_loss_ratio_bps > reqs.max_loss_ratio_bps {
        missing.push(format!(
            "Loss ratio {}bps above maximum {}bps",
            current_loss_ratio_bps, reqs.max_loss_ratio_bps
        ));
    }

    if consecutive_good_months < reqs.consecutive_good_months {
        missing.push(format!(
            "Need {} consecutive good months, have {}",
            reqs.consecutive_good_months, consecutive_good_months
        ));
    }

    if current_car_bps < reqs.min_car_bps {
        missing.push(format!(
            "CAR {}bps below minimum {}bps",
            current_car_bps, reqs.min_car_bps
        ));
    }

    if !reqs.audit_completed {
        missing.push("Smart contract audit not completed".to_string());
    }

    if !reqs.financial_audit_completed {
        missing.push("Financial audit not completed".to_string());
    }

    Ok(Phase1EligibilityStatus {
        eligible: missing.is_empty(),
        months_operating,
        months_required: reqs.min_months_operation,
        current_car_bps,
        required_car_bps: reqs.min_car_bps,
        audit_completed: reqs.audit_completed,
        financial_audit_completed: reqs.financial_audit_completed,
        missing_requirements: missing,
    })
}

// =============================================================================
// UPDATE PHASE REQUIREMENTS
// =============================================================================

/// Update Phase 1 requirements (DAO authority only)
#[derive(Accounts)]
pub struct UpdatePhase1Requirements<'info> {
    #[account(
        mut,
        seeds = [PhaseManager::SEED_PREFIX],
        bump = phase_manager.bump,
        constraint = phase_manager.authority == authority.key() @ ReservesError::Unauthorized
    )]
    pub phase_manager: Account<'info, PhaseManager>,

    pub authority: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdatePhase1Params {
    pub min_months_operation: Option<u8>,
    pub min_members: Option<u32>,
    pub min_loss_ratio_bps: Option<u16>,
    pub max_loss_ratio_bps: Option<u16>,
    pub consecutive_good_months: Option<u8>,
    pub min_car_bps: Option<u16>,
    pub audit_completed: Option<bool>,
    pub financial_audit_completed: Option<bool>,
}

pub fn update_phase1_requirements(
    ctx: Context<UpdatePhase1Requirements>,
    params: UpdatePhase1Params,
) -> Result<()> {
    let reqs = &mut ctx.accounts.phase_manager.phase1_requirements;

    if let Some(v) = params.min_months_operation {
        reqs.min_months_operation = v;
    }
    if let Some(v) = params.min_members {
        reqs.min_members = v;
    }
    if let Some(v) = params.min_loss_ratio_bps {
        reqs.min_loss_ratio_bps = v;
    }
    if let Some(v) = params.max_loss_ratio_bps {
        reqs.max_loss_ratio_bps = v;
    }
    if let Some(v) = params.consecutive_good_months {
        reqs.consecutive_good_months = v;
    }
    if let Some(v) = params.min_car_bps {
        reqs.min_car_bps = v;
    }
    if let Some(v) = params.audit_completed {
        reqs.audit_completed = v;
    }
    if let Some(v) = params.financial_audit_completed {
        reqs.financial_audit_completed = v;
    }

    emit!(PhaseRequirementsUpdated {
        phase: ProtocolPhase::Phase1Hcsm,
        updater: ctx.accounts.authority.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

// =============================================================================
// EXECUTE PHASE TRANSITION
// =============================================================================

/// Propose phase transition (starts DAO vote)
#[derive(Accounts)]
pub struct ProposePhaseTransition<'info> {
    #[account(
        mut,
        seeds = [PhaseManager::SEED_PREFIX],
        bump = phase_manager.bump,
        constraint = !phase_manager.transition_pending @ ReservesError::TransitionAlreadyPending,
        constraint = phase_manager.authority == authority.key() @ ReservesError::Unauthorized
    )]
    pub phase_manager: Account<'info, PhaseManager>,

    pub authority: Signer<'info>,
}

pub fn propose_phase_transition(
    ctx: Context<ProposePhaseTransition>,
    target_phase: ProtocolPhase,
) -> Result<()> {
    let manager = &mut ctx.accounts.phase_manager;

    // Validate transition is sequential
    let valid_transition = match (&manager.current_phase, &target_phase) {
        (ProtocolPhase::Phase1Hcsm, ProtocolPhase::Phase2Hybrid) => true,
        (ProtocolPhase::Phase2Hybrid, ProtocolPhase::Phase3Licensed) => true,
        _ => false,
    };

    require!(valid_transition, ReservesError::InvalidPhaseTransition);

    manager.transition_pending = true;
    manager.pending_target_phase = target_phase;

    emit!(PhaseTransitionProposed {
        current_phase: manager.current_phase,
        target_phase,
        proposer: ctx.accounts.authority.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

/// Execute approved phase transition (after DAO vote)
#[derive(Accounts)]
pub struct ExecutePhaseTransition<'info> {
    #[account(
        mut,
        seeds = [PhaseManager::SEED_PREFIX],
        bump = phase_manager.bump,
        constraint = phase_manager.transition_pending @ ReservesError::NoTransitionPending,
        constraint = phase_manager.authority == authority.key() @ ReservesError::Unauthorized
    )]
    pub phase_manager: Account<'info, PhaseManager>,

    pub authority: Signer<'info>,
}

pub fn execute_phase_transition(ctx: Context<ExecutePhaseTransition>) -> Result<()> {
    let clock = Clock::get()?;
    let manager = &mut ctx.accounts.phase_manager;

    let old_phase = manager.current_phase;
    let new_phase = manager.pending_target_phase;

    // Update phase timestamps
    match new_phase {
        ProtocolPhase::Phase2Hybrid => {
            manager.phase2_start = clock.unix_timestamp;
        }
        ProtocolPhase::Phase3Licensed => {
            manager.phase3_start = clock.unix_timestamp;
        }
        _ => {}
    }

    manager.current_phase = new_phase;
    manager.transition_pending = false;

    emit!(PhaseTransitionExecuted {
        old_phase,
        new_phase,
        executor: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Cancel pending phase transition
#[derive(Accounts)]
pub struct CancelPhaseTransition<'info> {
    #[account(
        mut,
        seeds = [PhaseManager::SEED_PREFIX],
        bump = phase_manager.bump,
        constraint = phase_manager.transition_pending @ ReservesError::NoTransitionPending,
        constraint = phase_manager.authority == authority.key() @ ReservesError::Unauthorized
    )]
    pub phase_manager: Account<'info, PhaseManager>,

    pub authority: Signer<'info>,
}

pub fn cancel_phase_transition(ctx: Context<CancelPhaseTransition>) -> Result<()> {
    let manager = &mut ctx.accounts.phase_manager;

    manager.transition_pending = false;

    emit!(PhaseTransitionCancelled {
        current_phase: manager.current_phase,
        cancelled_target: manager.pending_target_phase,
        canceller: ctx.accounts.authority.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

// =============================================================================
// REINSURANCE MANAGEMENT
// =============================================================================

/// Initialize reinsurance configuration
#[derive(Accounts)]
pub struct InitializeReinsurance<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + ReinsuranceConfig::INIT_SPACE,
        seeds = [ReinsuranceConfig::SEED_PREFIX],
        bump
    )]
    pub reinsurance_config: Account<'info, ReinsuranceConfig>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeReinsuranceParams {
    pub specific_attachment: u64,
    pub specific_coverage_bps: u16,
    pub aggregate_trigger_bps: u16,
    pub aggregate_ceiling_bps: u16,
    pub aggregate_coverage_bps: u16,
    pub policy_period_start: i64,
    pub policy_period_end: i64,
    pub expected_annual_claims: u64,
    pub reinsurance_premium: u64,
}

pub fn initialize_reinsurance(
    ctx: Context<InitializeReinsurance>,
    params: InitializeReinsuranceParams,
) -> Result<()> {
    let config = &mut ctx.accounts.reinsurance_config;

    config.authority = ctx.accounts.authority.key();
    config.specific_attachment = params.specific_attachment;
    config.specific_coverage_bps = params.specific_coverage_bps;
    config.aggregate_trigger_bps = params.aggregate_trigger_bps;
    config.aggregate_ceiling_bps = params.aggregate_ceiling_bps;
    config.aggregate_coverage_bps = params.aggregate_coverage_bps;
    config.policy_period_start = params.policy_period_start;
    config.policy_period_end = params.policy_period_end;
    config.expected_annual_claims = params.expected_annual_claims;
    config.specific_claims_paid = 0;
    config.specific_recovered = 0;
    config.aggregate_claims_accumulated = 0;
    config.aggregate_recovered = 0;
    config.reinsurance_premium_paid = params.reinsurance_premium;
    config.is_active = true;
    config.bump = ctx.bumps.reinsurance_config;

    emit!(ReinsuranceInitialized {
        specific_attachment: params.specific_attachment,
        aggregate_trigger_bps: params.aggregate_trigger_bps,
        policy_start: params.policy_period_start,
        policy_end: params.policy_period_end,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

/// Record a claim against reinsurance
#[derive(Accounts)]
pub struct RecordReinsuranceClaim<'info> {
    #[account(
        mut,
        seeds = [ReinsuranceConfig::SEED_PREFIX],
        bump = reinsurance_config.bump,
        constraint = reinsurance_config.is_active @ ReservesError::ReinsuranceInactive
    )]
    pub reinsurance_config: Account<'info, ReinsuranceConfig>,

    /// Authority (claims program or DAO)
    pub authority: Signer<'info>,
}

pub fn record_reinsurance_claim(
    ctx: Context<RecordReinsuranceClaim>,
    claim_amount: u64,
) -> Result<u64> {
    let config = &mut ctx.accounts.reinsurance_config;

    let mut recovery = 0u64;

    // Check specific stop-loss
    if config.triggers_specific(claim_amount) {
        let specific_recovery = config.calculate_specific_recovery(claim_amount);
        config.specific_claims_paid = config.specific_claims_paid.saturating_add(claim_amount);
        config.specific_recovered = config.specific_recovered.saturating_add(specific_recovery);
        recovery = recovery.saturating_add(specific_recovery);
    }

    // Accumulate for aggregate
    config.aggregate_claims_accumulated = config
        .aggregate_claims_accumulated
        .saturating_add(claim_amount);

    // Check aggregate stop-loss
    if config.triggers_aggregate() {
        // Aggregate recovery calculation would be more complex
        // For now, just track that it's triggered
        emit!(AggregateStopLossTriggered {
            accumulated_claims: config.aggregate_claims_accumulated,
            trigger_threshold: config.aggregate_trigger_bps,
            timestamp: Clock::get()?.unix_timestamp,
        });
    }

    if recovery > 0 {
        emit!(ReinsuranceRecovery {
            claim_amount,
            recovery_amount: recovery,
            timestamp: Clock::get()?.unix_timestamp,
        });
    }

    Ok(recovery)
}

// =============================================================================
// COHORT TRACKING
// =============================================================================

/// Initialize a new enrollment cohort
#[derive(Accounts)]
#[instruction(cohort_id: u32)]
pub struct InitializeCohort<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + CohortMetrics::INIT_SPACE,
        seeds = [CohortMetrics::SEED_PREFIX, &cohort_id.to_le_bytes()],
        bump
    )]
    pub cohort: Account<'info, CohortMetrics>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_cohort(ctx: Context<InitializeCohort>, cohort_id: u32) -> Result<()> {
    let cohort = &mut ctx.accounts.cohort;

    cohort.cohort_id = cohort_id;
    cohort.member_count = 0;
    cohort.active_count = 0;
    cohort.total_premiums = 0;
    cohort.total_claims = 0;
    cohort.loss_ratio_bps = 0;
    cohort.months_active = 0;
    cohort.flagged = false;
    cohort.bump = ctx.bumps.cohort;

    Ok(())
}

/// Update cohort metrics (called by membership/claims programs)
#[derive(Accounts)]
#[instruction(cohort_id: u32)]
pub struct UpdateCohortMetrics<'info> {
    #[account(
        mut,
        seeds = [CohortMetrics::SEED_PREFIX, &cohort_id.to_le_bytes()],
        bump = cohort.bump,
    )]
    pub cohort: Account<'info, CohortMetrics>,

    /// Authority (membership or claims program)
    pub authority: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CohortUpdateParams {
    pub add_members: Option<u32>,
    pub remove_members: Option<u32>,
    pub add_premiums: Option<u64>,
    pub add_claims: Option<u64>,
}

pub fn update_cohort_metrics(
    ctx: Context<UpdateCohortMetrics>,
    _cohort_id: u32,
    params: CohortUpdateParams,
) -> Result<()> {
    let cohort = &mut ctx.accounts.cohort;

    if let Some(add) = params.add_members {
        cohort.member_count = cohort.member_count.saturating_add(add);
        cohort.active_count = cohort.active_count.saturating_add(add);
    }

    if let Some(remove) = params.remove_members {
        cohort.active_count = cohort.active_count.saturating_sub(remove);
    }

    if let Some(premiums) = params.add_premiums {
        cohort.total_premiums = cohort.total_premiums.saturating_add(premiums);
    }

    if let Some(claims) = params.add_claims {
        cohort.total_claims = cohort.total_claims.saturating_add(claims);
    }

    // Recalculate loss ratio
    cohort.update_loss_ratio();

    if cohort.flagged {
        emit!(CohortFlagged {
            cohort_id: cohort.cohort_id,
            loss_ratio_bps: cohort.loss_ratio_bps,
            member_count: cohort.member_count,
            timestamp: Clock::get()?.unix_timestamp,
        });
    }

    Ok(())
}

// =============================================================================
// EVENTS
// =============================================================================

#[event]
pub struct PhaseManagerInitialized {
    pub authority: Pubkey,
    pub initial_phase: ProtocolPhase,
    pub timestamp: i64,
}

#[event]
pub struct PhaseRequirementsUpdated {
    pub phase: ProtocolPhase,
    pub updater: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct PhaseTransitionProposed {
    pub current_phase: ProtocolPhase,
    pub target_phase: ProtocolPhase,
    pub proposer: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct PhaseTransitionExecuted {
    pub old_phase: ProtocolPhase,
    pub new_phase: ProtocolPhase,
    pub executor: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct PhaseTransitionCancelled {
    pub current_phase: ProtocolPhase,
    pub cancelled_target: ProtocolPhase,
    pub canceller: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct ReinsuranceInitialized {
    pub specific_attachment: u64,
    pub aggregate_trigger_bps: u16,
    pub policy_start: i64,
    pub policy_end: i64,
    pub timestamp: i64,
}

#[event]
pub struct ReinsuranceRecovery {
    pub claim_amount: u64,
    pub recovery_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct AggregateStopLossTriggered {
    pub accumulated_claims: u64,
    pub trigger_threshold: u16,
    pub timestamp: i64,
}

#[event]
pub struct CohortFlagged {
    pub cohort_id: u32,
    pub loss_ratio_bps: u16,
    pub member_count: u32,
    pub timestamp: i64,
}
