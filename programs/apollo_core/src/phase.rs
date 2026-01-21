// programs/apollo_core/src/phase.rs
//
// Protocol Phase State Machine
// ============================
//
// Tracks Apollo Care's regulatory evolution through three phases:
// - Phase 1: Health Care Sharing Ministry (HCSM) - not insurance
// - Phase 2: Hybrid Model / Regulatory Sandbox - insurance pilot
// - Phase 3: Fully Licensed Insurer DAO
//
// Each phase has specific requirements, governance rules, and reserve targets.

use anchor_lang::prelude::*;

// =============================================================================
// PHASE DEFINITIONS
// =============================================================================

/// Protocol operational phase
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
#[repr(u8)]
pub enum ProtocolPhase {
    /// Phase 1: Health Care Sharing Ministry
    /// - Not legally insurance
    /// - Self-regulated via DAO
    /// - Voluntary cost sharing
    /// - Operates under HCSM exemptions
    Phase1Hcsm = 1,
    
    /// Phase 2: Hybrid Model / Regulatory Sandbox
    /// - Insurance pilot in select states
    /// - HCSM continues in parallel
    /// - Working with regulators
    /// - Building compliance infrastructure
    Phase2Hybrid = 2,
    
    /// Phase 3: Fully Licensed Insurer
    /// - Licensed insurance carrier
    /// - Full regulatory compliance
    /// - Statutory reserve requirements
    /// - Guaranty fund participation
    Phase3Licensed = 3,
}

impl Default for ProtocolPhase {
    fn default() -> Self {
        ProtocolPhase::Phase1Hcsm
    }
}

impl ProtocolPhase {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            ProtocolPhase::Phase1Hcsm => "Health Care Sharing Ministry",
            ProtocolPhase::Phase2Hybrid => "Hybrid Model / Regulatory Sandbox",
            ProtocolPhase::Phase3Licensed => "Licensed Insurance DAO",
        }
    }
    
    /// Get recommended minimum CAR for this phase
    pub fn min_car_bps(&self) -> u16 {
        match self {
            ProtocolPhase::Phase1Hcsm => 12500,  // 125%
            ProtocolPhase::Phase2Hybrid => 15000, // 150%
            ProtocolPhase::Phase3Licensed => 20000, // 200%
        }
    }
    
    /// Get recommended reserve days (Tier 1) for this phase
    pub fn recommended_tier1_days(&self) -> u16 {
        match self {
            ProtocolPhase::Phase1Hcsm => 60,   // 2 months
            ProtocolPhase::Phase2Hybrid => 90,  // 3 months
            ProtocolPhase::Phase3Licensed => 120, // 4 months (per statute)
        }
    }
    
    /// Check if this phase requires external regulatory approval
    pub fn requires_regulatory_approval(&self) -> bool {
        match self {
            ProtocolPhase::Phase1Hcsm => false,
            ProtocolPhase::Phase2Hybrid => true,
            ProtocolPhase::Phase3Licensed => true,
        }
    }
    
    /// Get next phase (if applicable)
    pub fn next_phase(&self) -> Option<ProtocolPhase> {
        match self {
            ProtocolPhase::Phase1Hcsm => Some(ProtocolPhase::Phase2Hybrid),
            ProtocolPhase::Phase2Hybrid => Some(ProtocolPhase::Phase3Licensed),
            ProtocolPhase::Phase3Licensed => None,
        }
    }
}

// =============================================================================
// PHASE STATE ACCOUNT
// =============================================================================

/// Protocol phase tracking state
/// PDA seeds: ["protocol_phase"]
#[account]
#[derive(InitSpace)]
pub struct ProtocolPhaseState {
    /// Current operational phase
    pub current_phase: ProtocolPhase,
    
    /// Phase 1 start timestamp
    pub phase1_started_at: i64,
    
    /// Phase 2 start timestamp (0 if not started)
    pub phase2_started_at: i64,
    
    /// Phase 3 start timestamp (0 if not started)
    pub phase3_started_at: i64,
    
    /// Authority that can propose phase transitions
    pub transition_authority: Pubkey,
    
    /// Governance program for transition votes
    pub governance_program: Pubkey,
    
    /// Required vote threshold for transitions (basis points)
    /// Phase 1→2: 66.67% (6667 bps)
    /// Phase 2→3: 75% (7500 bps)
    pub transition_vote_threshold_bps: u16,
    
    /// Compliance flags for regulatory readiness
    pub compliance: PhaseComplianceFlags,
    
    /// Primary jurisdiction (state code)
    #[max_len(2)]
    pub primary_jurisdiction: String,
    
    /// Licensed entity name (Phase 3)
    #[max_len(64)]
    pub licensed_entity_name: String,
    
    /// Is a transition currently in progress (pending vote)?
    pub transition_in_progress: bool,
    
    /// Proposed target phase (if transition in progress)
    pub proposed_phase: Option<ProtocolPhase>,
    
    /// Transition proposal timestamp
    pub transition_proposed_at: i64,
    
    /// Bump seed
    pub bump: u8,
}

impl ProtocolPhaseState {
    pub const SEED_PREFIX: &'static [u8] = b"protocol_phase";
    
    // Default vote thresholds
    pub const PHASE_1_TO_2_VOTE_BPS: u16 = 6667; // 2/3 majority
    pub const PHASE_2_TO_3_VOTE_BPS: u16 = 7500; // 75% supermajority
    
    /// Get the required vote threshold for transitioning to target phase
    pub fn get_transition_threshold(target: ProtocolPhase) -> u16 {
        match target {
            ProtocolPhase::Phase1Hcsm => 0, // Can't transition to Phase 1
            ProtocolPhase::Phase2Hybrid => Self::PHASE_1_TO_2_VOTE_BPS,
            ProtocolPhase::Phase3Licensed => Self::PHASE_2_TO_3_VOTE_BPS,
        }
    }
    
    /// Check if transition from current phase to target is valid
    pub fn is_valid_transition(&self, target: ProtocolPhase) -> bool {
        match (&self.current_phase, &target) {
            (ProtocolPhase::Phase1Hcsm, ProtocolPhase::Phase2Hybrid) => true,
            (ProtocolPhase::Phase2Hybrid, ProtocolPhase::Phase3Licensed) => true,
            _ => false, // No skipping phases, no going backwards
        }
    }
    
    /// Get time in current phase (seconds)
    pub fn time_in_current_phase(&self, current_time: i64) -> i64 {
        let phase_start = match self.current_phase {
            ProtocolPhase::Phase1Hcsm => self.phase1_started_at,
            ProtocolPhase::Phase2Hybrid => self.phase2_started_at,
            ProtocolPhase::Phase3Licensed => self.phase3_started_at,
        };
        
        current_time.saturating_sub(phase_start)
    }
}

// =============================================================================
// COMPLIANCE FLAGS
// =============================================================================

/// Compliance tracking for phase transitions
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, InitSpace)]
pub struct PhaseComplianceFlags {
    // ========== Phase 1 (HCSM) Compliance ==========
    
    /// HCSM disclaimer enabled on all member communications
    pub hcsm_disclaimer_enabled: bool,
    
    /// Voluntary sharing terms accepted by all members
    pub voluntary_sharing_terms: bool,
    
    /// Wyoming DAO LLC in good standing
    pub wyoming_dao_compliant: bool,
    
    // ========== Phase 2 (Hybrid) Compliance ==========
    
    /// Regulatory sandbox/pilot license obtained
    pub sandbox_license_obtained: bool,
    
    /// Pilot state(s) approved
    pub pilot_state_approved: bool,
    
    /// Insurance policy forms filed with regulator
    pub policy_forms_filed: bool,
    
    /// Rate filings submitted to regulator
    pub rate_filings_submitted: bool,
    
    /// External actuarial audit completed
    pub actuarial_audit_completed: bool,
    
    /// Financial audit completed
    pub financial_audit_completed: bool,
    
    // ========== Phase 3 (Licensed) Compliance ==========
    
    /// Full insurance license obtained
    pub full_insurance_license: bool,
    
    /// Participating in state guaranty fund
    pub guaranty_fund_member: bool,
    
    /// Statutory capital requirements met
    pub statutory_capital_met: bool,
    
    /// All rate filings approved by regulator
    pub all_filings_approved: bool,
    
    /// Board of Directors established
    pub board_established: bool,
    
    /// Named executive officers appointed
    pub officers_appointed: bool,
}

impl PhaseComplianceFlags {
    /// Check if ready for Phase 2 transition
    pub fn ready_for_phase2(&self) -> bool {
        self.hcsm_disclaimer_enabled
            && self.voluntary_sharing_terms
            && self.wyoming_dao_compliant
            && self.actuarial_audit_completed
            && self.financial_audit_completed
    }
    
    /// Check if ready for Phase 3 transition
    pub fn ready_for_phase3(&self) -> bool {
        self.ready_for_phase2()
            && self.sandbox_license_obtained
            && self.pilot_state_approved
            && self.policy_forms_filed
            && self.rate_filings_submitted
    }
}

// =============================================================================
// PHASE TRANSITION REQUIREMENTS
// =============================================================================

/// Requirements for Phase 1 → Phase 2 transition
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct Phase1To2Requirements {
    /// Minimum members for transition
    pub min_members: u64,
    
    /// Minimum months of Phase 1 operation
    pub min_months_operating: u8,
    
    /// Minimum MLR demonstrated over period
    pub min_demonstrated_mlr_bps: u16,
    
    /// Minimum CAR maintained throughout
    pub min_car_maintained_bps: u16,
    
    /// Minimum Tier 1 reserve days
    pub min_tier1_days: u16,
}

impl Default for Phase1To2Requirements {
    fn default() -> Self {
        Self {
            min_members: 1_000,           // Start small for small-scale viability
            min_months_operating: 12,      // 1 year track record
            min_demonstrated_mlr_bps: 8500, // 85%+ MLR demonstrated
            min_car_maintained_bps: 12500,  // 125%+ CAR maintained
            min_tier1_days: 90,             // 3 months Tier 1 reserves
        }
    }
}

/// Requirements for Phase 2 → Phase 3 transition
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct Phase2To3Requirements {
    /// Minimum members for transition
    pub min_members: u64,
    
    /// Minimum months of Phase 2 operation
    pub min_pilot_months: u8,
    
    /// Regulatory examination passed
    pub regulatory_exam_passed: bool,
    
    /// Statutory capital requirements met
    pub statutory_capital_met: bool,
    
    /// All state filings approved
    pub all_filings_approved: bool,
    
    /// Minimum CAR for transition
    pub min_car_bps: u16,
}

impl Default for Phase2To3Requirements {
    fn default() -> Self {
        Self {
            min_members: 10_000,           // Meaningful scale
            min_pilot_months: 24,          // 2 year pilot minimum
            regulatory_exam_passed: false,
            statutory_capital_met: false,
            all_filings_approved: false,
            min_car_bps: 15000,            // 150%+ CAR
        }
    }
}

// =============================================================================
// EVENTS
// =============================================================================

#[event]
pub struct PhaseTransitionProposed {
    pub from_phase: ProtocolPhase,
    pub to_phase: ProtocolPhase,
    pub proposed_at: i64,
    pub proposer: Pubkey,
    pub vote_threshold_bps: u16,
}

#[event]
pub struct PhaseTransitionCompleted {
    pub from_phase: ProtocolPhase,
    pub to_phase: ProtocolPhase,
    pub completed_at: i64,
    pub vote_result_bps: u16,
}

#[event]
pub struct PhaseTransitionRejected {
    pub from_phase: ProtocolPhase,
    pub to_phase: ProtocolPhase,
    pub rejected_at: i64,
    pub vote_result_bps: u16,
    pub threshold_required_bps: u16,
}

#[event]
pub struct ComplianceFlagUpdated {
    pub flag_name: String,
    pub new_value: bool,
    pub updated_at: i64,
    pub updater: Pubkey,
}

// =============================================================================
// UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_phase_defaults() {
        assert_eq!(ProtocolPhase::default(), ProtocolPhase::Phase1Hcsm);
    }
    
    #[test]
    fn test_phase_min_car() {
        assert_eq!(ProtocolPhase::Phase1Hcsm.min_car_bps(), 12500);
        assert_eq!(ProtocolPhase::Phase2Hybrid.min_car_bps(), 15000);
        assert_eq!(ProtocolPhase::Phase3Licensed.min_car_bps(), 20000);
    }
    
    #[test]
    fn test_phase_transitions() {
        let phase1 = ProtocolPhase::Phase1Hcsm;
        let phase2 = ProtocolPhase::Phase2Hybrid;
        let phase3 = ProtocolPhase::Phase3Licensed;
        
        assert_eq!(phase1.next_phase(), Some(phase2));
        assert_eq!(phase2.next_phase(), Some(phase3));
        assert_eq!(phase3.next_phase(), None);
    }
    
    #[test]
    fn test_regulatory_approval_required() {
        assert!(!ProtocolPhase::Phase1Hcsm.requires_regulatory_approval());
        assert!(ProtocolPhase::Phase2Hybrid.requires_regulatory_approval());
        assert!(ProtocolPhase::Phase3Licensed.requires_regulatory_approval());
    }
    
    #[test]
    fn test_compliance_flags_ready_for_phase2() {
        let mut flags = PhaseComplianceFlags::default();
        assert!(!flags.ready_for_phase2());
        
        flags.hcsm_disclaimer_enabled = true;
        flags.voluntary_sharing_terms = true;
        flags.wyoming_dao_compliant = true;
        flags.actuarial_audit_completed = true;
        assert!(!flags.ready_for_phase2()); // Still missing financial audit
        
        flags.financial_audit_completed = true;
        assert!(flags.ready_for_phase2());
    }
    
    #[test]
    fn test_transition_thresholds() {
        assert_eq!(
            ProtocolPhaseState::get_transition_threshold(ProtocolPhase::Phase2Hybrid),
            6667 // 2/3 majority
        );
        assert_eq!(
            ProtocolPhaseState::get_transition_threshold(ProtocolPhase::Phase3Licensed),
            7500 // 75% supermajority
        );
    }
    
    #[test]
    fn test_valid_transitions() {
        let state = ProtocolPhaseState {
            current_phase: ProtocolPhase::Phase1Hcsm,
            phase1_started_at: 0,
            phase2_started_at: 0,
            phase3_started_at: 0,
            transition_authority: Pubkey::default(),
            governance_program: Pubkey::default(),
            transition_vote_threshold_bps: 6667,
            compliance: PhaseComplianceFlags::default(),
            primary_jurisdiction: String::from("WY"),
            licensed_entity_name: String::new(),
            transition_in_progress: false,
            proposed_phase: None,
            transition_proposed_at: 0,
            bump: 255,
        };
        
        // Valid: Phase 1 → Phase 2
        assert!(state.is_valid_transition(ProtocolPhase::Phase2Hybrid));
        
        // Invalid: Phase 1 → Phase 3 (can't skip)
        assert!(!state.is_valid_transition(ProtocolPhase::Phase3Licensed));
        
        // Invalid: Phase 1 → Phase 1
        assert!(!state.is_valid_transition(ProtocolPhase::Phase1Hcsm));
    }
    
    #[test]
    fn test_phase1_to_2_requirements_defaults() {
        let req = Phase1To2Requirements::default();
        assert_eq!(req.min_members, 1_000);
        assert_eq!(req.min_months_operating, 12);
        assert_eq!(req.min_demonstrated_mlr_bps, 8500);
        assert_eq!(req.min_car_maintained_bps, 12500);
        assert_eq!(req.min_tier1_days, 90);
    }
    
    #[test]
    fn test_phase2_to_3_requirements_defaults() {
        let req = Phase2To3Requirements::default();
        assert_eq!(req.min_members, 10_000);
        assert_eq!(req.min_pilot_months, 24);
        assert_eq!(req.min_car_bps, 15000);
    }
}
