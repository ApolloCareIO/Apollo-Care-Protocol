# Apollo Care Protocol: Bootstrap Guide

> Comprehensive guide for operating Apollo Care at bootstrap scale ($1.5M-$5M capital)

**Last Updated**: January 2026  
**Status**: Ready for Implementation

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Capital Scenarios](#capital-scenarios)
3. [Smart Contract Parameters](#smart-contract-parameters)
4. [AI/ML Claims Processing](#aiml-claims-processing)
5. [Reinsurance Strategy](#reinsurance-strategy)
6. [Phase Transitions](#phase-transitions)
7. [Implementation Status](#implementation-status)
8. [Risk Mitigation](#risk-mitigation)

---

## Executive Summary

Apollo Care Protocol is viable at bootstrap scale with targeted adjustments. The core smart contract architecture is sound; the primary changes are operational parameters, not structural redesigns.

**Key Findings:**
- Protocol can support 200-500 members at $1.5M capital
- Reinsurance is **critical** (not optional) at small scale
- Three-tier claims processing enables 60-70% auto-approval
- Phase 1→2→3 transition is encoded in smart contracts

---

## Capital Scenarios

### Scenario Comparison

| Scenario | Capital | Members | Monthly Claims | Monthly Premiums |
|----------|---------|---------|----------------|------------------|
| **Original ICO** | $50M | 10,000+ | $4.2M | $4.5M |
| **Soft Cap** | $1.5M | 200-500 | $84k-$210k | $90k-$225k |
| **Realistic** | $3-5M | 500-1,500 | $210k-$630k | $225k-$675k |

### Small Scale Risk Profile

Higher **Objective Risk** at small scale (per Morrisey):
```
Objective Risk = σ / (μ√N)
```
- N=500 members: Risk is ~14x higher than N=10,000
- N=200 members: Risk is ~22x higher than N=10,000

**Implications:**
1. Claims variance much higher month-to-month
2. Single catastrophic claim has larger relative impact
3. More reliance on reinsurance to smooth volatility
4. Tighter enrollment controls critical

### Day 1 Allocation ($1.5M)

| Use | Amount | % |
|-----|--------|---|
| Tier 2 Reserve | $750k | 50% |
| Operations (12mo) | $450k | 30% |
| Reinsurance Premium | $150k | 10% |
| Legal/Compliance | $100k | 7% |
| Security Audit | $50k | 3% |

**Capacity**: ~200 members in Year 1 with reinsurance

---

## Smart Contract Parameters

### Bootstrap Mode Adjustments

#### ClaimsConfig (`apollo_claims/src/state.rs`)

```rust
// Standard scale defaults (10,000+ members)
pub const DEFAULT_AUTO_APPROVE: u64 = 1_000_000_000; // $1,000
pub const DEFAULT_SHOCK_THRESHOLD: u64 = 100_000_000_000; // $100,000

// Bootstrap scale defaults (< 1,000 members)
pub const BOOTSTRAP_AUTO_APPROVE: u64 = 500_000_000; // $500
pub const BOOTSTRAP_SHOCK_THRESHOLD: u64 = 25_000_000_000; // $25,000

// Dynamic threshold calculation
pub const SHOCK_THRESHOLD_BPS: u16 = 500; // 5% of reserves
pub const SHOCK_THRESHOLD_MIN: u64 = 10_000_000_000; // $10k floor
pub const SHOCK_THRESHOLD_MAX: u64 = 100_000_000_000; // $100k ceiling
```

**Rationale**: At $1.5M capital with 300 members, a $100k claim is ~7% of total capital. Lower thresholds trigger human review earlier.

#### ReserveConfig (`apollo_reserves/src/state.rs`)

```rust
// Standard scale
pub const DEFAULT_TIER0_DAYS: u16 = 30;
pub const DEFAULT_TIER1_DAYS: u16 = 60;
pub const DEFAULT_TIER2_DAYS: u16 = 180;

// Bootstrap scale - extend Tier 0 for liquidity cushion
pub const BOOTSTRAP_TIER0_DAYS: u16 = 45;
pub const BOOTSTRAP_TIER1_DAYS: u16 = 60;
pub const BOOTSTRAP_TIER2_DAYS: u16 = 120;
```

#### Enrollment Caps (`apollo_risk_engine/src/state.rs`)

Bootstrap enrollment throttling (even in Green Zone):
- Green: 100/month max
- Yellow: 50/month max
- Orange: 20/month max
- Red: Frozen

---

## AI/ML Claims Processing

### Three-Tier Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                 TIER 1: FAST-LANE AUTO-APPROVAL                 │
│  • Amount ≤ $500 (bootstrap) / $1,000 (scaled)                  │
│  • Category: PrimaryCare, Prescription, Lab, Preventive         │
│  • Member in good standing, ≤5 fast-lane claims/30 days         │
│  Target: 60-70% of claims                                       │
└─────────────────────────────┬───────────────────────────────────┘
                              │ Not eligible
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                 TIER 2: AI-ASSISTED TRIAGE                      │
│  • Price reasonableness (vs UCR database)                       │
│  • Procedure-diagnosis consistency                              │
│  • Fraud pattern detection                                      │
│  Outputs: AUTO_APPROVE | COMMITTEE_REVIEW | AUTO_DENY           │
│  Target: 25-30% of claims, 80% auto-resolved                    │
└─────────────────────────────┬───────────────────────────────────┘
                              │ Flagged/Complex
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                 TIER 3: COMMITTEE ESCALATION                    │
│  • Amount > $25k shock threshold                                │
│  • AI confidence < 70%                                          │
│  • Appeals                                                      │
│  Target: 5-10% of claims                                        │
└─────────────────────────────────────────────────────────────────┘
```

### Bootstrap AI Strategy

| Phase | Members | Approach |
|-------|---------|----------|
| **Phase 1** | 0-500 | Rule-based only (UCR lookups, hard-coded fraud flags) |
| **Phase 2** | 500-2,000 | Hybrid (ML models + rules fallback) |
| **Phase 3** | 2,000+ | Full ML with continuous learning |

### Oracle Structure

```rust
#[account]
pub struct ClaimsOracle {
    pub authorized_signers: Vec<Pubkey>,
    pub required_sigs: u8,
    pub total_decisions: u64,
    pub decisions_overturned: u64,
    pub accuracy_rate_bps: u16,
    pub is_active: bool,
    pub bump: u8,
}
```

---

## Reinsurance Strategy

At small scale, reinsurance is **essential for survival**, not optional.

### Recommended Structure

| Layer | Attachment | Coverage | Est. Cost |
|-------|------------|----------|-----------|
| Specific | $50k per claim | 90% | 2-3% of premium |
| Aggregate | 110% of expected | 100% to 150% | 3-4% of premium |
| Catastrophic | 150% of expected | 100% to 300% | 1-2% of premium |

**Total**: ~6-9% of premium revenue

### Example: Catastrophic Claim Scenario

$150k cancer treatment claim in Month 3 with $1.5M capital:

| Scenario | Apollo Pays | CAR Impact |
|----------|-------------|------------|
| **Without Reinsurance** | $150k (10% of capital) | Drops to ~90%, Red Zone |
| **With Reinsurance ($50k attachment)** | $50k (3.3% of capital) | ~5% drop, operations continue |

---

## Phase Transitions

### Overview

| Phase | Status | Timeline | Members | Capital |
|-------|--------|----------|---------|---------|
| **Phase 1** | HCSM | Months 1-12 | 100-500 | $1.5M-$3M |
| **Phase 2** | Hybrid/Sandbox | Months 12-24 | 500-2,000 | $3M-$10M |
| **Phase 3** | Licensed | Year 2+ | 2,000+ | $10M+ |

### Phase 1 → 2 Requirements

```rust
pub struct Phase1To2Requirements {
    pub min_months_operation: u8,      // 12
    pub min_members: u32,              // 300
    pub min_loss_ratio_bps: u16,       // 8500 (85%)
    pub max_loss_ratio_bps: u16,       // 9500 (95%)
    pub consecutive_good_months: u8,   // 6
    pub min_car_bps: u16,              // 12500 (125%)
    pub audit_completed: bool,
}
```

### Phase 2 → 3 Requirements

```rust
pub struct Phase2To3Requirements {
    pub min_months_sandbox: u8,        // 12
    pub min_members: u32,              // 1000
    pub regulatory_approval: bool,
    pub statutory_capital_met: bool,
    pub actuarial_certification: bool,
}
```

### Smart Contract Configuration by Phase

```rust
// Phase 1 settings
PhaseConfig {
    phase: Phase::CostSharingMinistry,
    max_members: 500,
    enrollment_requires_attestation: true,
    auto_approve_threshold: 500_000_000,    // $500
    shock_claim_threshold: 25_000_000_000,  // $25k
    waiting_period_days: 30,
    min_car_for_enrollment: 12500,          // 125%
}
```

---

## Implementation Status

### ✅ Implemented

| Component | Status | Notes |
|-----------|--------|-------|
| Bootstrap thresholds | ✅ | ClaimsConfig, ReserveConfig |
| Three-tier claims flow | ✅ | Fast-lane, AI triage, committee |
| AI Oracle account | ✅ | Multi-sig oracle support |
| Phase manager | ✅ | Phase enum and transitions |
| Enrollment throttling | ✅ | Zone-based caps |
| Reinsurance config | ✅ | ReinsurancePool in apollo_reinsurance |

### 🔄 In Progress

| Component | Status | Notes |
|-----------|--------|-------|
| Rule-based fraud detection | 🔄 | Basic flags implemented |
| UCR price database | 🔄 | Off-chain integration needed |
| Regulatory compliance module | 🔄 | Phase 2 requirement |

### 📋 Planned

| Component | Priority | Phase |
|-----------|----------|-------|
| ML model training pipeline | P1 | Phase 2 |
| Statutory reporting | P2 | Phase 2-3 |
| Multi-state licensing | P2 | Phase 3 |

---

## Risk Mitigation

### Adverse Selection

**Mitigations in code:**
- 30-day waiting period
- Enhanced screening in Yellow/Orange zones
- Enrollment throttling
- Cohort loss ratio monitoring

### Smart Contract Exploit

**Mitigations:**
- Reentrancy guards
- Access control checks
- Emergency pause functionality
- Multi-sig for parameter changes
- 24-hour timelock on large withdrawals

### Liquidity Crisis

**Mitigations:**
- Extended Tier 0 buffer (45 days vs 30)
- Reinsurance from Day 1
- Gradual release from cold storage
- CAR-based enrollment freezes

---

## Quick Reference

### Key Constants (Bootstrap Mode)

| Parameter | Value | Location |
|-----------|-------|----------|
| Auto-approve threshold | $500 | `apollo_claims/state.rs` |
| Shock claim threshold | $25k | `apollo_claims/state.rs` |
| Tier 0 buffer | 45 days | `apollo_reserves/state.rs` |
| Min CAR for enrollment | 125% | `apollo_risk_engine/state.rs` |
| Waiting period | 30 days | `apollo_membership/state.rs` |

### Program IDs

| Program | Address |
|---------|---------|
| apollo_core | See `Anchor.toml` |
| apollo_claims | See `Anchor.toml` |
| apollo_reserves | See `Anchor.toml` |
| apollo_membership | See `Anchor.toml` |

---

*Consolidated from: BOOTSTRAP_VIABILITY_ANALYSIS.md, BOOTSTRAP_VIABILITY_SPEC.md, SMALL_SCALE_VIABILITY_AUDIT.md, IMPLEMENTATION_CHANGES_SUMMARY.md, IMPLEMENTATION_STATUS.md, IMPLEMENTATION_SUMMARY.md*
