# Apollo Care Protocol: Implementation Changes Summary

## Overview

This document summarizes all code changes made to adapt the Apollo Care Protocol for bootstrap-scale operation ($1.5M-$5M), implement AI/ML claims processing, and add phase transition mechanics.

---

## 1. Bootstrap Mode Support

### File: `programs/apollo_claims/src/state.rs`

**Added bootstrap-aware threshold functions:**
```rust
impl ClaimsConfig {
    // Standard scale defaults (10,000+ members)
    pub const DEFAULT_AUTO_APPROVE: u64 = 1_000_000_000; // $1,000
    pub const DEFAULT_SHOCK_THRESHOLD: u64 = 100_000_000_000; // $100,000
    
    // Bootstrap scale defaults (< 1,000 members)
    pub const BOOTSTRAP_AUTO_APPROVE: u64 = 500_000_000; // $500
    pub const BOOTSTRAP_SHOCK_THRESHOLD: u64 = 25_000_000_000; // $25,000
    
    // Helper functions to get appropriate thresholds
    pub fn get_auto_approve_threshold(member_count: u32) -> u64
    pub fn get_shock_threshold(member_count: u32) -> u64
}
```

**Rationale**: At bootstrap scale, claims variance is much higher. Lower thresholds trigger human review earlier to protect the small pool.

---

## 2. AI/ML Claims Processing System

### New File: `programs/apollo_claims/src/instructions/ai_processing.rs`

**New Accounts:**

| Account | Purpose |
|---------|---------|
| `AiOracle` | Configuration for off-chain ML decision system |
| `AiDecision` | Records AI decision for a specific claim |
| `FastLaneUsage` | Tracks member's monthly fast-lane usage |

**Three-Tier Processing:**

1. **Fast-Lane (Tier 1)**: Instant auto-approval for small routine claims
   - Claims ≤ $500 (bootstrap) / $1,000 (scaled)
   - Eligible categories: PrimaryCare, Prescription, Laboratory, Preventive, DiagnosticImaging
   - Max 5 fast-lane claims per member per month

2. **AI Triage (Tier 2)**: ML-assisted decision for medium claims
   - Confidence scoring (0-100%)
   - Fraud risk scoring
   - Price reasonableness scoring
   - Auto-approve if confidence ≥ 95% AND fraud score ≤ 30%
   - Escalate to committee if confidence < 70%

3. **Committee Review (Tier 3)**: Human review for large/complex claims
   - Claims > $25k (bootstrap shock threshold)
   - AI confidence < 70%
   - Flagged claims

**New Instructions:**
- `initialize_ai_oracle`: Set up oracle with authorized signers
- `submit_ai_decision`: Oracle submits ML decision for a claim
- `process_fast_lane`: Instant approval for eligible small claims
- `mark_decision_overturned`: Track AI accuracy when committee overturns

---

## 3. Phase Transition Management

### File: `programs/apollo_reserves/src/state.rs`

**New Accounts:**

| Account | Purpose |
|---------|---------|
| `PhaseManager` | Tracks protocol phase and transition requirements |
| `ReinsuranceConfig` | Reinsurance policy configuration |
| `CohortMetrics` | Adverse selection tracking by enrollment cohort |

**Protocol Phases:**
```rust
pub enum ProtocolPhase {
    CostSharingMinistry,  // Phase 1: HCSM (not insurance)
    HybridSandbox,        // Phase 2: HCSM + Regulatory pilot
    LicensedInsurer,      // Phase 3: Full insurance license
}
```

**Phase 1 → 2 Requirements:**
- 12 months successful operation
- ≥ 300 active members
- Loss ratio between 85-95% for 6 consecutive months
- CAR maintained ≥ 125%
- Smart contract audit completed
- Financial audit completed

**Phase 2 → 3 Requirements:**
- 12 months in sandbox
- ≥ 1,000 active members
- Regulatory approval received
- Statutory capital requirements met
- Actuarial certification obtained
- All committees established

### New File: `programs/apollo_reserves/src/instructions/phase_management.rs`

**New Instructions:**
- `initialize_phase_manager`: Start protocol in Phase 1
- `check_phase1_eligibility`: Verify transition requirements
- `update_phase1_requirements`: DAO updates requirements
- `propose_phase_transition`: Start DAO vote for transition
- `execute_phase_transition`: Execute approved transition
- `cancel_phase_transition`: Cancel pending transition
- `initialize_reinsurance`: Set up reinsurance policy
- `record_reinsurance_claim`: Track claims against reinsurance
- `initialize_cohort`: Create enrollment cohort
- `update_cohort_metrics`: Track cohort loss ratios

---

## 4. Reinsurance Support

### File: `programs/apollo_reserves/src/state.rs`

```rust
pub struct ReinsuranceConfig {
    // Specific stop-loss (per claim)
    pub specific_attachment: u64,      // $50k bootstrap / $100k standard
    pub specific_coverage_bps: u16,    // 90%
    
    // Aggregate stop-loss (annual total)
    pub aggregate_trigger_bps: u16,    // 110% of expected
    pub aggregate_ceiling_bps: u16,    // 150% of expected
    pub aggregate_coverage_bps: u16,   // 100%
    
    // Tracking
    pub specific_claims_paid: u64,
    pub specific_recovered: u64,
    pub aggregate_claims_accumulated: u64,
    pub aggregate_recovered: u64,
}
```

**Bootstrap Defaults** (more protective for small pools):
- Specific attachment: $50,000 (vs $100k standard)
- Triggers earlier to protect limited capital

---

## 5. Cohort-Based Adverse Selection Monitoring

### File: `programs/apollo_reserves/src/state.rs`

```rust
pub struct CohortMetrics {
    pub cohort_id: u32,           // YYYYMM format
    pub member_count: u32,
    pub active_count: u32,
    pub total_premiums: u64,
    pub total_claims: u64,
    pub loss_ratio_bps: u16,
    pub flagged: bool,            // Auto-flagged if > 120% loss ratio
}
```

**Purpose**: Track whether specific enrollment cohorts are experiencing adverse selection. If a cohort's loss ratio exceeds 120%, it's flagged for investigation.

---

## 6. Error Handling Updates

### File: `programs/apollo_claims/src/errors.rs`

**Added errors:**
- `OracleInactive`: AI Oracle is not active
- `InvalidConfiguration`: Invalid oracle setup
- `ExceedsFastLaneLimit`: Claim too large for fast-lane
- `CategoryNotEligible`: Category not eligible for fast-lane
- `FastLaneLimitExceeded`: Monthly fast-lane limit reached
- `DecisionAlreadyRecorded`: AI decision exists for claim
- `AlreadyOverturned`: Decision already overturned
- `FraudDetected`: High fraud probability

### File: `programs/apollo_reserves/src/errors.rs`

**Added errors:**
- `InvalidPhaseTransition`: Non-sequential transition
- `TransitionAlreadyPending`: Transition in progress
- `NoTransitionPending`: No transition to execute
- `PhaseRequirementsNotMet`: Requirements not satisfied
- `ReinsuranceInactive`: Reinsurance not active
- `ReinsurancePolicyExpired`: Policy period ended
- `InvalidReinsuranceConfig`: Bad reinsurance params
- `CohortNotFound`: Cohort doesn't exist
- `CohortFlagged`: Cohort has adverse selection

---

## 7. Module Updates

### File: `programs/apollo_claims/src/instructions/mod.rs`
```rust
pub mod ai_processing;
pub use ai_processing::*;
```

### File: `programs/apollo_reserves/src/instructions/mod.rs`
```rust
pub mod phase_management;
pub use phase_management::*;
```

### File: `programs/apollo_claims/src/lib.rs`
Updated to include new AI processing instructions.

---

## 8. Documentation

### New File: `docs/BOOTSTRAP_VIABILITY_ANALYSIS.md`

Comprehensive 500+ line analysis covering:
- Capital scenario comparison ($50M vs $1.5M)
- Objective risk calculations at small scale
- Smart contract parameter recommendations
- Three-tier AI claims processing architecture
- ML model specifications
- Off-chain oracle architecture
- Phase 1→2→3 transition mechanics
- Risk mitigation strategies
- Implementation priorities

---

## Files Modified

| File | Changes |
|------|---------|
| `programs/apollo_claims/src/state.rs` | Bootstrap constants, helper functions |
| `programs/apollo_claims/src/errors.rs` | AI processing errors |
| `programs/apollo_claims/src/lib.rs` | New instruction exports |
| `programs/apollo_claims/src/instructions/mod.rs` | ai_processing module |
| `programs/apollo_reserves/src/state.rs` | Reinsurance, PhaseManager, Cohort accounts |
| `programs/apollo_reserves/src/errors.rs` | Phase/reinsurance errors |
| `programs/apollo_reserves/src/instructions/mod.rs` | phase_management module |

## Files Created

| File | Purpose |
|------|---------|
| `programs/apollo_claims/src/instructions/ai_processing.rs` | AI oracle and fast-lane processing |
| `programs/apollo_reserves/src/instructions/phase_management.rs` | Phase transitions, reinsurance, cohorts |
| `docs/BOOTSTRAP_VIABILITY_ANALYSIS.md` | Comprehensive analysis document |

---

## Next Steps

1. **Pre-Launch (P0)**:
   - Finalize reinsurance partner negotiations
   - Complete smart contract audit
   - Configure bootstrap thresholds

2. **Phase 1 Development (P1)**:
   - Build rule-based fraud detection (before ML data exists)
   - Integrate UCR price database
   - Deploy cohort tracking

3. **Phase 2 Development**:
   - Train ML models on Phase 1 data
   - Build regulatory compliance module
   - Prepare statutory reporting

---

*Generated: 2026-01-19*
