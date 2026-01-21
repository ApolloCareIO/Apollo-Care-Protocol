# Apollo Care Protocol: Bootstrap Viability Specification

## Executive Summary

This document addresses Apollo Care's viability at **bootstrap scale** ($1.5M-$5M capital) versus the originally envisioned $50M ICO. It covers:

1. **Small-Scale Viability** - Parameter adjustments for lean operations
2. **AI/ML Claims Processing** - Core product differentiation
3. **Phase 1→2→3 Transition Mechanics** - Regulatory pathway

---

## Part 1: Small-Scale Viability Analysis

### 1.1 Capital Requirements at Bootstrap

**Scenario: $1.5M Soft Cap**

| Use Case | Allocation | Amount | Rationale |
|----------|------------|--------|-----------|
| Operating Reserves (Tier 1) | 50% | $750K | 3 months claims @ 100 members |
| Liquidity Buffer (Tier 0) | 15% | $225K | 2-4 weeks instant payouts |
| Operations Runway | 20% | $300K | 12-18 months lean ops |
| Legal/Compliance | 10% | $150K | Phase 1 HCSM structure |
| Security/Audits | 5% | $75K | Smart contract audit |

**Key Insight**: At $1.5M, Apollo can safely cover ~100-200 members in Phase 1.

### 1.2 Member Capacity Calculation

```
Expected Annual Claims per Member: $4,000-$6,000 (healthy population)
Monthly Claims per Member: $333-$500
With $750K Tier 1 Reserve:
  Conservative: $750K / $500 = 1,500 member-months
  At 100 members: 15 months reserve runway
  At 200 members: 7.5 months reserve runway
```

**Recommendation**: Cap initial enrollment at 150 members until premium inflow stabilizes reserves.

### 1.3 Current Codebase Audit - Scale Issues

| Module | Current State | Issue | Fix Required |
|--------|---------------|-------|--------------|
| `ClaimsConfig::DEFAULT_AUTO_APPROVE` | $1,000 | ✅ OK | None |
| `ClaimsConfig::DEFAULT_SHOCK_THRESHOLD` | $100,000 | ⚠️ High for bootstrap | Reduce to $25K |
| `RiskConfig::DEFAULT_BASE_RATE` | $450/month | ✅ OK | None |
| Enrollment throttling | Fixed 500/month Yellow | ⚠️ Too high | Dynamic based on reserves |
| Reserve K1 multiplier | 3.0 | ⚠️ May be aggressive | Make dynamic |

### 1.4 Required Parameter Changes for Bootstrap Mode

```rust
// NEW: Bootstrap mode configuration
pub mod bootstrap {
    /// Bootstrap mode flag - enables conservative parameters
    pub const BOOTSTRAP_MODE: bool = true;
    
    /// Maximum members in bootstrap phase
    pub const BOOTSTRAP_MAX_MEMBERS: u32 = 200;
    
    /// Shock claim threshold during bootstrap (lower = more scrutiny)
    pub const BOOTSTRAP_SHOCK_THRESHOLD: u64 = 25_000_000_000; // $25K
    
    /// Auto-approve threshold during bootstrap (conservative)
    pub const BOOTSTRAP_AUTO_APPROVE: u64 = 500_000_000; // $500
    
    /// Minimum reserve days before accepting new members
    pub const BOOTSTRAP_MIN_RESERVE_DAYS: u16 = 90;
    
    /// Enrollment throttle in bootstrap (members per month)
    pub const BOOTSTRAP_ENROLLMENT_CAP: u32 = 25;
}
```

### 1.5 Dynamic Enrollment Gates

Instead of fixed CAR zone thresholds, enrollment should be tied to **absolute reserve coverage**:

```rust
pub fn calculate_enrollment_capacity(
    tier0_balance: u64,
    tier1_balance: u64,
    expected_monthly_claims: u64,
    current_members: u32,
) -> EnrollmentCapacity {
    let total_liquid = tier0_balance + tier1_balance;
    let months_coverage = total_liquid / expected_monthly_claims.max(1);
    
    match months_coverage {
        12.. => EnrollmentCapacity::Unlimited,
        6..=11 => EnrollmentCapacity::Throttled(50),  // 50/month
        3..=5 => EnrollmentCapacity::Throttled(10),   // 10/month
        _ => EnrollmentCapacity::Frozen,
    }
}
```

### 1.6 Reinsurance at Bootstrap Scale

**Problem**: Traditional reinsurers won't write policies for <500 lives.

**Solution for Phase 1**:
1. **Higher deductibles** ($2,500-$5,000) to reduce claim frequency
2. **Annual maximum per member** ($100K) to cap exposure
3. **Aggregate stop-loss via DAO treasury** - if claims exceed 120% of expected, draw from Tier 2
4. **Partner with a captive or fronting insurer** for catastrophic coverage

---

## Part 2: AI/ML Claims Processing System

### 2.1 Three-Tier Architecture (Per Whitepaper)

```
┌─────────────────────────────────────────────────────────────────┐
│                    CLAIMS SUBMISSION                            │
│                         ↓                                       │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  TIER 1: FAST-LANE AUTO-APPROVAL                        │   │
│  │  • Claims < $500 (bootstrap) / $1,000 (scaled)          │   │
│  │  • Eligible categories (primary care, generic Rx, labs) │   │
│  │  • Member in good standing                              │   │
│  │  • No flags in fraud scoring                            │   │
│  │  → INSTANT PAYOUT (seconds)                             │   │
│  └─────────────────────────────────────────────────────────┘   │
│                         ↓ (if not fast-lane)                    │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  TIER 2: AI-ASSISTED TRIAGE                             │   │
│  │  • ML fraud scoring (0-100 risk score)                  │   │
│  │  • Price reasonableness check (vs reference database)   │   │
│  │  • Document validation (OCR + verification)             │   │
│  │  • Member history analysis                              │   │
│  │  → AUTO-APPROVE if score < 30 (minutes)                 │   │
│  │  → FLAG for review if score 30-70                       │   │
│  │  → ESCALATE if score > 70                               │   │
│  └─────────────────────────────────────────────────────────┘   │
│                         ↓ (if flagged/escalated)                │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  TIER 3: COMMITTEE/DAO REVIEW                           │   │
│  │  • Claims Committee (2+ attestations)                   │   │
│  │  • 48-hour resolution target                            │   │
│  │  • DAO vote for shock claims (>$25K bootstrap)          │   │
│  │  → HUMAN DECISION (1-7 days)                            │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 AI Oracle Integration

The on-chain contracts need to integrate with an off-chain AI service. Here's the architecture:

```rust
/// AI Oracle configuration
/// PDA seeds: ["ai_oracle_config"]
#[account]
pub struct AiOracleConfig {
    /// Oracle authority (can submit AI decisions)
    pub oracle_authority: Pubkey,
    
    /// Backup oracle (if primary fails)
    pub backup_oracle: Pubkey,
    
    /// Maximum staleness for AI decisions (seconds)
    pub max_decision_age: i64,
    
    /// Auto-approve threshold score (0-100, lower = safer)
    pub auto_approve_threshold: u8,
    
    /// Escalation threshold score (0-100, higher = riskier)
    pub escalation_threshold: u8,
    
    /// Is AI processing enabled
    pub is_enabled: bool,
    
    /// Total decisions processed
    pub total_decisions: u64,
    
    /// Decisions overridden by committee
    pub overridden_count: u64,
    
    pub bump: u8,
}

/// AI decision for a specific claim
/// PDA seeds: ["ai_decision", claim_id]
#[account]
pub struct AiDecision {
    /// Claim this decision applies to
    pub claim_id: u64,
    
    /// Risk score (0-100, higher = riskier)
    pub risk_score: u8,
    
    /// Fraud indicators detected
    pub fraud_flags: FraudFlags,
    
    /// Price reasonableness score (0-100, higher = more reasonable)
    pub price_score: u8,
    
    /// Recommended action
    pub recommendation: AiRecommendation,
    
    /// Recommended amount (may differ from requested)
    pub recommended_amount: u64,
    
    /// Confidence level (0-100)
    pub confidence: u8,
    
    /// Reasoning hash (IPFS link to detailed explanation)
    pub reasoning_hash: [u8; 32],
    
    /// Timestamp of decision
    pub decided_at: i64,
    
    /// Was this decision overridden by committee
    pub was_overridden: bool,
    
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct FraudFlags {
    /// Duplicate claim detected
    pub duplicate_claim: bool,
    /// Price significantly above reference
    pub price_anomaly: bool,
    /// Unusual claim frequency
    pub frequency_anomaly: bool,
    /// Document issues detected
    pub document_issues: bool,
    /// Provider on watchlist
    pub provider_flagged: bool,
    /// Service doesn't match diagnosis
    pub service_mismatch: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum AiRecommendation {
    /// Auto-approve in full
    AutoApproveFull,
    /// Auto-approve partial amount
    AutoApprovePartial,
    /// Needs human review (medium risk)
    NeedsReview,
    /// High risk - escalate to committee
    Escalate,
    /// Likely fraud - deny with investigation
    DenyFraud,
}
```

### 2.3 AI Processing Flow (Off-Chain)

```python
# Simplified AI claims processor (off-chain service)

class ClaimsAIProcessor:
    def __init__(self, reference_db, fraud_model, ocr_service):
        self.reference_db = reference_db
        self.fraud_model = fraud_model
        self.ocr_service = ocr_service
    
    async def process_claim(self, claim_data: dict) -> AiDecision:
        # 1. Extract document data
        doc_data = await self.ocr_service.extract(claim_data['documents'])
        
        # 2. Validate against coverage rules
        coverage_valid = self.validate_coverage(
            claim_data['member_id'],
            claim_data['service_code'],
            claim_data['service_date']
        )
        
        # 3. Check price reasonableness
        reference_price = self.reference_db.get_price(
            claim_data['service_code'],
            claim_data['region']
        )
        price_score = self.calculate_price_score(
            claim_data['amount'],
            reference_price
        )
        
        # 4. Run fraud detection model
        fraud_features = self.extract_fraud_features(claim_data, doc_data)
        fraud_score = self.fraud_model.predict(fraud_features)
        
        # 5. Check member history
        history_flags = self.analyze_member_history(claim_data['member_id'])
        
        # 6. Calculate composite risk score
        risk_score = self.calculate_risk_score(
            fraud_score,
            price_score,
            history_flags,
            coverage_valid
        )
        
        # 7. Generate recommendation
        return self.generate_decision(risk_score, claim_data)
    
    def calculate_risk_score(self, fraud, price, history, coverage) -> int:
        """
        Composite scoring:
        - Fraud model: 40% weight
        - Price anomaly: 25% weight
        - History flags: 20% weight
        - Coverage issues: 15% weight
        """
        if not coverage:
            return 100  # Auto-deny if not covered
        
        score = (
            fraud * 0.40 +
            (100 - price) * 0.25 +
            history * 0.20 +
            0  # Coverage already validated
        )
        return min(100, max(0, int(score)))
```

### 2.4 Fraud Detection Features

The ML model should consider:

| Feature Category | Specific Features |
|-----------------|-------------------|
| **Claim Patterns** | Frequency, amount distribution, timing |
| **Provider Analysis** | Claims/provider ratio, specialty match, location |
| **Document Signals** | OCR confidence, template matching, alteration detection |
| **Member Behavior** | Claims history, enrollment recency, payment history |
| **Network Effects** | Provider-member relationships, referral patterns |
| **Price Signals** | Deviation from UCR, regional variance, code matching |

### 2.5 Reference Price Database

Apollo needs a reference price database for cost reasonableness checks:

```rust
/// Reference price entry
#[account]
pub struct ReferencePrice {
    /// CPT/HCPCS code
    pub service_code: [u8; 8],
    
    /// Region code
    pub region: u8,
    
    /// 25th percentile price (USDC)
    pub p25_price: u64,
    
    /// 50th percentile (median) price
    pub p50_price: u64,
    
    /// 75th percentile price
    pub p75_price: u64,
    
    /// 95th percentile (high but acceptable)
    pub p95_price: u64,
    
    /// Last updated
    pub updated_at: i64,
    
    /// Data source hash
    pub source_hash: [u8; 32],
}
```

**Data Sources** (Phase 1):
- CMS Medicare fee schedules (public)
- FAIR Health consumer cost data
- Member-reported prices (crowdsourced)

---

## Part 3: Phase Transition Mechanics

### 3.1 Phase State Machine

```rust
/// Protocol phase state
/// PDA seeds: ["phase_state"]
#[account]
pub struct PhaseState {
    /// Current operating phase
    pub current_phase: ProtocolPhase,
    
    /// Phase entered timestamp
    pub phase_entered_at: i64,
    
    /// Phase transition requirements
    pub transition_requirements: TransitionRequirements,
    
    /// Regulatory status
    pub regulatory_status: RegulatoryStatus,
    
    /// Authority for phase changes (DAO)
    pub authority: Pubkey,
    
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolPhase {
    /// Phase 1: Health Care Sharing Ministry
    /// - Not insurance, voluntary cost-sharing
    /// - No state licenses required (in most states)
    /// - "Not insurance" disclaimer required
    CostSharingMinistry = 0,
    
    /// Phase 2: Regulatory Sandbox / Hybrid
    /// - Pilot insurance license in 1-2 states
    /// - Dual operation (HCSM + regulated in pilot states)
    /// - Working with regulators
    RegulatoryPilot = 1,
    
    /// Phase 3: Licensed Insurer
    /// - Full insurance licenses
    /// - Compliance with state requirements
    /// - Guaranty fund participation
    LicensedInsurer = 2,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TransitionRequirements {
    /// Minimum members for phase transition
    pub min_members: u32,
    
    /// Minimum months of operation
    pub min_operating_months: u16,
    
    /// Minimum capital (USDC)
    pub min_capital: u64,
    
    /// Required CAR for transition
    pub min_car_bps: u16,
    
    /// Loss ratio must be below this (bps)
    pub max_loss_ratio_bps: u16,
    
    /// Requires DAO supermajority vote
    pub requires_supermajority: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RegulatoryStatus {
    /// States where licensed (Phase 2+)
    pub licensed_states: [u8; 8], // Bitmap for 50 states
    
    /// Pending applications
    pub pending_applications: u8,
    
    /// Regulatory sandbox active
    pub sandbox_active: bool,
    
    /// Wyoming DAO LLC status
    pub wyoming_dao_active: bool,
}
```

### 3.2 Phase Transition Requirements

| Transition | Requirements | Governance |
|------------|--------------|------------|
| **Phase 1 → 2** | 1,000+ members, 12+ months operation, $2M+ capital, <100% loss ratio | DAO supermajority (67%) |
| **Phase 2 → 3** | 5,000+ members, regulatory approval in 3+ states, $10M+ capital | DAO supermajority + regulatory sign-off |

### 3.3 Phase-Specific Behaviors

```rust
impl PhaseState {
    /// Get maximum individual claim for this phase
    pub fn max_individual_claim(&self) -> u64 {
        match self.current_phase {
            ProtocolPhase::CostSharingMinistry => 100_000_000_000, // $100K
            ProtocolPhase::RegulatoryPilot => 250_000_000_000,     // $250K
            ProtocolPhase::LicensedInsurer => 1_000_000_000_000,   // $1M
        }
    }
    
    /// Check if pre-existing condition exclusions allowed
    pub fn preexisting_exclusions_allowed(&self) -> bool {
        match self.current_phase {
            ProtocolPhase::CostSharingMinistry => true,  // HCSMs can exclude
            _ => false,  // ACA-compliant = no exclusions
        }
    }
    
    /// Required disclaimers for member communications
    pub fn required_disclaimers(&self) -> Vec<&'static str> {
        match self.current_phase {
            ProtocolPhase::CostSharingMinistry => vec![
                "This is not insurance.",
                "Sharing of medical expenses is voluntary.",
                "There is no guarantee that your expenses will be shared.",
                "This program is not subject to state insurance regulations.",
            ],
            ProtocolPhase::RegulatoryPilot => vec![
                "Coverage may vary by state.",
                "Check your state's specific terms.",
            ],
            ProtocolPhase::LicensedInsurer => vec![], // Standard insurance disclosures
        }
    }
    
    /// Contribution terminology for phase
    pub fn contribution_term(&self) -> &'static str {
        match self.current_phase {
            ProtocolPhase::CostSharingMinistry => "monthly share amount",
            _ => "premium",
        }
    }
}
```

### 3.4 Phase Transition Instruction

```rust
/// Propose phase transition (requires DAO vote to execute)
pub fn propose_phase_transition(
    ctx: Context<ProposePhaseTransition>,
    target_phase: ProtocolPhase,
) -> Result<()> {
    let phase_state = &ctx.accounts.phase_state;
    let current = phase_state.current_phase;
    
    // Validate transition is sequential
    require!(
        (current as u8) + 1 == (target_phase as u8),
        PhaseError::InvalidTransition
    );
    
    // Validate requirements met
    let reqs = &phase_state.transition_requirements;
    let membership = &ctx.accounts.membership_state;
    let reserves = &ctx.accounts.reserve_state;
    
    require!(
        membership.active_members >= reqs.min_members,
        PhaseError::InsufficientMembers
    );
    
    require!(
        reserves.total_capital() >= reqs.min_capital,
        PhaseError::InsufficientCapital
    );
    
    // Create governance proposal
    // ... (CPI to governance program)
    
    Ok(())
}
```

---

## Part 4: Implementation Roadmap

### 4.1 Immediate Priorities (Week 1-2)

1. **Add Bootstrap Mode Constants**
   - Lower thresholds for claims
   - Conservative enrollment caps
   - Higher scrutiny parameters

2. **Add Phase State Management**
   - PhaseState account
   - Phase-specific behaviors
   - Transition proposal logic

3. **Add AI Oracle Scaffolding**
   - AiOracleConfig account
   - AiDecision account
   - Oracle submission instruction

### 4.2 Short-Term (Week 3-4)

4. **Implement AI Decision Flow**
   - Off-chain AI processor skeleton
   - On-chain decision recording
   - Integration with claims workflow

5. **Add Reference Price Infrastructure**
   - ReferencePrice account
   - Price reasonableness checks
   - Initial data seeding

### 4.3 Medium-Term (Month 2)

6. **Fraud Detection Integration**
   - FraudFlags structure
   - Pattern detection rules
   - Provider watchlist

7. **Phase Transition Mechanics**
   - Transition requirements validation
   - DAO proposal integration
   - Regulatory status tracking

---

## Part 5: Testing Scenarios

### 5.1 Bootstrap Scale Tests

```typescript
describe("Bootstrap Viability", () => {
  it("should limit enrollment at low reserves", async () => {
    // Set reserves to 2 months coverage
    await setReserveBalance(reserves, 150_000_000_000); // $150K
    await setExpectedMonthlyClaims(75_000_000_000); // $75K
    
    // Should only allow 10/month enrollment
    const capacity = await program.getEnrollmentCapacity();
    expect(capacity.monthlyLimit).to.equal(10);
  });
  
  it("should use lower shock threshold in bootstrap", async () => {
    // Enable bootstrap mode
    await enableBootstrapMode();
    
    // Submit $30K claim
    const result = await submitClaim(30_000_000_000);
    
    // Should be flagged as shock claim in bootstrap
    expect(result.isShockClaim).to.be.true;
  });
});
```

### 5.2 AI Processing Tests

```typescript
describe("AI Claims Processing", () => {
  it("should auto-approve low-risk claims", async () => {
    // Submit AI decision with low risk score
    await submitAiDecision(claimId, {
      riskScore: 15,
      recommendation: AiRecommendation.AutoApproveFull,
      confidence: 95,
    });
    
    // Claim should be auto-approved
    const claim = await getClaim(claimId);
    expect(claim.status).to.equal(ClaimStatus.Approved);
  });
  
  it("should escalate high-risk claims", async () => {
    await submitAiDecision(claimId, {
      riskScore: 85,
      recommendation: AiRecommendation.Escalate,
      fraudFlags: { priceAnomaly: true, frequencyAnomaly: true },
    });
    
    const claim = await getClaim(claimId);
    expect(claim.status).to.equal(ClaimStatus.PendingAttestation);
  });
});
```

### 5.3 Phase Transition Tests

```typescript
describe("Phase Transitions", () => {
  it("should reject transition with insufficient members", async () => {
    // Set membership below threshold
    await setActiveMemberCount(500); // Below 1000 requirement
    
    await expect(
      proposePhaseTransition(ProtocolPhase.RegulatoryPilot)
    ).to.be.rejectedWith("InsufficientMembers");
  });
  
  it("should allow transition when requirements met", async () => {
    await setActiveMemberCount(1500);
    await setOperatingMonths(14);
    await setTotalCapital(3_000_000_000_000); // $3M
    
    const proposalId = await proposePhaseTransition(ProtocolPhase.RegulatoryPilot);
    expect(proposalId).to.not.be.null;
  });
});
```

---

## Conclusion

Apollo Care can operate viably at bootstrap scale ($1.5M-$5M) with:

1. **Conservative parameters** that auto-adjust based on reserve levels
2. **AI-first claims processing** that enables 90%+ MLR through automation
3. **Clear phase transitions** from HCSM → Pilot → Licensed Insurer

The key is focusing on **product utility** (claims processing, member experience, cost savings) rather than capital-intensive growth. Phase 1 proves the model with ~100-200 members, building the track record needed for Phase 2 regulatory engagement.

**Next Steps**:
1. Implement bootstrap mode constants
2. Add AI oracle integration to claims module
3. Add phase state management
4. Create off-chain AI processor skeleton

---

*Document Version: 1.0*
*Date: 2026-01-19*
*Status: SPECIFICATION*
