// programs/apollo_reserves/src/instructions/phase.rs
//
// Phase Transition Instructions
// Manages protocol evolution from HCSM → Hybrid → Licensed Insurer

use anchor_lang::prelude::*;
use crate::state::{
    PhaseManager, ProtocolPhase, Phase1Requirements, Phase2Requirements,
    ReserveConfig, ReserveState,
};
use crate::errors::ReservesError;

// =============================================================================
// INITIALIZATION
// =============================================================================

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

    #[account(
        seeds = [ReserveConfig::SEED_PREFIX],
        bump = reserve_config.bump,
        constraint = reserve_config.authority == authority.key() @ ReservesError::Unauthorized
    )]
    pub reserve_config: Account<'info, ReserveConfig>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_phase_manager(ctx: Context<InitializePhaseManager>) -> Result<()> {
    let clock = Clock::get()?;
    let phase_manager = &mut ctx.accounts.phase_manager;
    
    phase_manager.authority = ctx.accounts.authority.key();
    phase_manager.current_phase = ProtocolPhase::CostSharingMinistry;
    phase_manager.phase1_start = clock.unix_timestamp;
    phase_manager.phase2_start = 0;
    phase_manager.phase3_start = 0;
    phase_manager.phase1_requirements = Phase1Requirements::default();
    phase_manager.phase2_requirements = Phase2Requirements::default();
    phase_manager.transition_pending = false;
    phase_manager.pending_target_phase = ProtocolPhase::CostSharingMinistry;
    phase_manager.bump = ctx.bumps.phase_manager;
    
    emit!(PhaseManagerInitialized {
        authority: ctx.accounts.authority.key(),
        initial_phase: ProtocolPhase::CostSharingMinistry,
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}

// =============================================================================
// UPDATE PHASE REQUIREMENTS
// =============================================================================

#[derive(Accounts)]
pub struct UpdatePhaseRequirements<'info> {
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
pub struct UpdatePhase1RequirementsParams {
    pub min_months_operation: Option<u8>,
    pub min_members: Option<u32>,
    pub min_loss_ratio_bps: Option<u16>,
    pub max_loss_ratio_bps: Option<u16>,
    pub consecutive_good_months: Option<u8>,
    pub min_car_bps: Option<u16>,
}

pub fn update_phase1_requirements(
    ctx: Context<UpdatePhaseRequirements>,
    params: UpdatePhase1RequirementsParams,
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
    
    Ok(())
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdatePhase2RequirementsParams {
    pub min_months_sandbox: Option<u8>,
    pub min_members: Option<u32>,
}

pub fn update_phase2_requirements(
    ctx: Context<UpdatePhaseRequirements>,
    params: UpdatePhase2RequirementsParams,
) -> Result<()> {
    let reqs = &mut ctx.accounts.phase_manager.phase2_requirements;
    
    if let Some(v) = params.min_months_sandbox {
        reqs.min_months_sandbox = v;
    }
    if let Some(v) = params.min_members {
        reqs.min_members = v;
    }
    
    Ok(())
}

// =============================================================================
// AUDIT COMPLETION
// =============================================================================

#[derive(Accounts)]
pub struct MarkAuditComplete<'info> {
    #[account(
        mut,
        seeds = [PhaseManager::SEED_PREFIX],
        bump = phase_manager.bump,
        constraint = phase_manager.authority == authority.key() @ ReservesError::Unauthorized
    )]
    pub phase_manager: Account<'info, PhaseManager>,

    pub authority: Signer<'info>,
}

pub fn mark_smart_contract_audit_complete(ctx: Context<MarkAuditComplete>) -> Result<()> {
    ctx.accounts.phase_manager.phase1_requirements.audit_completed = true;
    
    emit!(AuditCompleted {
        audit_type: AuditType::SmartContract,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    Ok(())
}

pub fn mark_financial_audit_complete(ctx: Context<MarkAuditComplete>) -> Result<()> {
    ctx.accounts.phase_manager.phase1_requirements.financial_audit_completed = true;
    
    emit!(AuditCompleted {
        audit_type: AuditType::Financial,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    Ok(())
}

// =============================================================================
// REGULATORY MILESTONES (Phase 2 → Phase 3)
// =============================================================================

#[derive(Accounts)]
pub struct MarkRegulatoryMilestone<'info> {
    #[account(
        mut,
        seeds = [PhaseManager::SEED_PREFIX],
        bump = phase_manager.bump,
        constraint = phase_manager.authority == authority.key() @ ReservesError::Unauthorized,
        constraint = phase_manager.current_phase == ProtocolPhase::HybridSandbox @ ReservesError::InvalidPhase
    )]
    pub phase_manager: Account<'info, PhaseManager>,

    pub authority: Signer<'info>,
}

pub fn mark_regulatory_approval(ctx: Context<MarkRegulatoryMilestone>) -> Result<()> {
    ctx.accounts.phase_manager.phase2_requirements.regulatory_approval = true;
    
    emit!(RegulatoryMilestone {
        milestone: RegulatoryMilestoneType::Approval,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    Ok(())
}

pub fn mark_statutory_capital_met(ctx: Context<MarkRegulatoryMilestone>) -> Result<()> {
    ctx.accounts.phase_manager.phase2_requirements.statutory_capital_met = true;
    
    emit!(RegulatoryMilestone {
        milestone: RegulatoryMilestoneType::StatutoryCapital,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    Ok(())
}

pub fn mark_actuarial_certification(ctx: Context<MarkRegulatoryMilestone>) -> Result<()> {
    ctx.accounts.phase_manager.phase2_requirements.actuarial_certification = true;
    
    emit!(RegulatoryMilestone {
        milestone: RegulatoryMilestoneType::ActuarialCertification,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    Ok(())
}

pub fn mark_committees_established(ctx: Context<MarkRegulatoryMilestone>) -> Result<()> {
    ctx.accounts.phase_manager.phase2_requirements.committees_established = true;
    
    emit!(RegulatoryMilestone {
        milestone: RegulatoryMilestoneType::CommitteesEstablished,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    Ok(())
}

// =============================================================================
// CHECK TRANSITION ELIGIBILITY
// =============================================================================

#[derive(Accounts)]
pub struct CheckPhaseTransition<'info> {
    #[account(
        seeds = [PhaseManager::SEED_PREFIX],
        bump = phase_manager.bump,
    )]
    pub phase_manager: Account<'info, PhaseManager>,

    #[account(
        seeds = [ReserveState::SEED_PREFIX],
        bump,
    )]
    pub reserve_state: Account<'info, ReserveState>,
    
    // In production, would also include:
    // - Membership state (member count)
    // - Risk engine (CAR)
    // - Loss ratio metrics
}

/// Check result for phase transition eligibility
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct TransitionCheckResult {
    pub eligible: bool,
    pub current_phase: ProtocolPhase,
    pub target_phase: ProtocolPhase,
    pub missing_requirements: Vec<String>,
}

pub fn check_transition_eligibility(
    ctx: Context<CheckPhaseTransition>,
    active_members: u32,
    current_car_bps: u16,
    loss_ratio_bps: u16,
    consecutive_good_months: u8,
) -> Result<TransitionCheckResult> {
    let clock = Clock::get()?;
    let manager = &ctx.accounts.phase_manager;
    
    let mut missing = Vec::new();
    
    match manager.current_phase {
        ProtocolPhase::CostSharingMinistry => {
            let reqs = &manager.phase1_requirements;
            
            // Check months of operation
            let months_operating = (clock.unix_timestamp - manager.phase1_start) / (30 * 86400);
            if months_operating < reqs.min_months_operation as i64 {
                missing.push(format!(
                    "Need {} months operation, have {}",
                    reqs.min_months_operation, months_operating
                ));
            }
            
            // Check members
            if active_members < reqs.min_members {
                missing.push(format!(
                    "Need {} members, have {}",
                    reqs.min_members, active_members
                ));
            }
            
            // Check loss ratio
            if loss_ratio_bps < reqs.min_loss_ratio_bps || loss_ratio_bps > reqs.max_loss_ratio_bps {
                missing.push(format!(
                    "Loss ratio {} outside range {}-{}",
                    loss_ratio_bps, reqs.min_loss_ratio_bps, reqs.max_loss_ratio_bps
                ));
            }
            
            // Check consecutive good months
            if consecutive_good_months < reqs.consecutive_good_months {
                missing.push(format!(
                    "Need {} consecutive good months, have {}",
                    reqs.consecutive_good_months, consecutive_good_months
                ));
            }
            
            // Check CAR
            if current_car_bps < reqs.min_car_bps {
                missing.push(format!(
                    "CAR {} below minimum {}",
                    current_car_bps, reqs.min_car_bps
                ));
            }
            
            // Check audits
            if !reqs.audit_completed {
                missing.push("Smart contract audit not completed".to_string());
            }
            if !reqs.financial_audit_completed {
                missing.push("Financial audit not completed".to_string());
            }
            
            Ok(TransitionCheckResult {
                eligible: missing.is_empty(),
                current_phase: ProtocolPhase::CostSharingMinistry,
                target_phase: ProtocolPhase::HybridSandbox,
                missing_requirements: missing,
            })
        },
        
        ProtocolPhase::HybridSandbox => {
            let reqs = &manager.phase2_requirements;
            
            // Check months in sandbox
            let months_sandbox = (clock.unix_timestamp - manager.phase2_start) / (30 * 86400);
            if months_sandbox < reqs.min_months_sandbox as i64 {
                missing.push(format!(
                    "Need {} months in sandbox, have {}",
                    reqs.min_months_sandbox, months_sandbox
                ));
            }
            
            // Check members
            if active_members < reqs.min_members {
                missing.push(format!(
                    "Need {} members, have {}",
                    reqs.min_members, active_members
                ));
            }
            
            // Check regulatory milestones
            if !reqs.regulatory_approval {
                missing.push("Regulatory approval not received".to_string());
            }
            if !reqs.statutory_capital_met {
                missing.push("Statutory capital requirements not met".to_string());
            }
            if !reqs.actuarial_certification {
                missing.push("Actuarial certification not obtained".to_string());
            }
            if !reqs.committees_established {
                missing.push("Required committees not established".to_string());
            }
            
            Ok(TransitionCheckResult {
                eligible: missing.is_empty(),
                current_phase: ProtocolPhase::HybridSandbox,
                target_phase: ProtocolPhase::LicensedInsurer,
                missing_requirements: missing,
            })
        },
        
        ProtocolPhase::LicensedInsurer => {
            Ok(TransitionCheckResult {
                eligible: false,
                current_phase: ProtocolPhase::LicensedInsurer,
                target_phase: ProtocolPhase::LicensedInsurer,
                missing_requirements: vec!["Already at final phase".to_string()],
            })
        },
    }
}

// =============================================================================
// EXECUTE PHASE TRANSITION
// =============================================================================

#[derive(Accounts)]
pub struct ExecutePhaseTransition<'info> {
    #[account(
        mut,
        seeds = [PhaseManager::SEED_PREFIX],
        bump = phase_manager.bump,
        constraint = phase_manager.authority == authority.key() @ ReservesError::Unauthorized
    )]
    pub phase_manager: Account<'info, PhaseManager>,

    pub authority: Signer<'info>,
}

pub fn execute_transition_to_phase2(ctx: Context<ExecutePhaseTransition>) -> Result<()> {
    let clock = Clock::get()?;
    let manager = &mut ctx.accounts.phase_manager;
    
    require!(
        manager.current_phase == ProtocolPhase::CostSharingMinistry,
        ReservesError::InvalidPhase
    );
    
    // Verify all requirements (caller should have checked first)
    let reqs = &manager.phase1_requirements;
    require!(reqs.audit_completed, ReservesError::AuditNotComplete);
    require!(reqs.financial_audit_completed, ReservesError::AuditNotComplete);
    
    // Execute transition
    let old_phase = manager.current_phase;
    manager.current_phase = ProtocolPhase::HybridSandbox;
    manager.phase2_start = clock.unix_timestamp;
    manager.transition_pending = false;
    
    emit!(PhaseTransitioned {
        from_phase: old_phase,
        to_phase: ProtocolPhase::HybridSandbox,
        timestamp: clock.unix_timestamp,
        transitioner: ctx.accounts.authority.key(),
    });
    
    Ok(())
}

pub fn execute_transition_to_phase3(ctx: Context<ExecutePhaseTransition>) -> Result<()> {
    let clock = Clock::get()?;
    let manager = &mut ctx.accounts.phase_manager;
    
    require!(
        manager.current_phase == ProtocolPhase::HybridSandbox,
        ReservesError::InvalidPhase
    );
    
    // Verify all requirements
    let reqs = &manager.phase2_requirements;
    require!(reqs.regulatory_approval, ReservesError::RegulatoryApprovalRequired);
    require!(reqs.statutory_capital_met, ReservesError::InsufficientCapital);
    require!(reqs.actuarial_certification, ReservesError::CertificationRequired);
    require!(reqs.committees_established, ReservesError::CommitteesRequired);
    
    // Execute transition
    let old_phase = manager.current_phase;
    manager.current_phase = ProtocolPhase::LicensedInsurer;
    manager.phase3_start = clock.unix_timestamp;
    manager.transition_pending = false;
    
    emit!(PhaseTransitioned {
        from_phase: old_phase,
        to_phase: ProtocolPhase::LicensedInsurer,
        timestamp: clock.unix_timestamp,
        transitioner: ctx.accounts.authority.key(),
    });
    
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
pub struct PhaseTransitioned {
    pub from_phase: ProtocolPhase,
    pub to_phase: ProtocolPhase,
    pub timestamp: i64,
    pub transitioner: Pubkey,
}

#[event]
pub struct AuditCompleted {
    pub audit_type: AuditType,
    pub timestamp: i64,
}

#[event]
pub struct RegulatoryMilestone {
    pub milestone: RegulatoryMilestoneType,
    pub timestamp: i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum AuditType {
    SmartContract,
    Financial,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum RegulatoryMilestoneType {
    Approval,
    StatutoryCapital,
    ActuarialCertification,
    CommitteesEstablished,
}
