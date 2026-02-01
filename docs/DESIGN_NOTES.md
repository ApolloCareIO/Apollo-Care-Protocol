# Apollo Care Protocol: Design Notes

> Supplementary design documentation covering actuarial foundations and AI/ML architecture

**Last Updated**: January 2026

---

## Table of Contents

1. [Actuarial Framework](#actuarial-framework)
2. [AI/ML Architecture](#aiml-architecture)
3. [Data Flow](#data-flow)
4. [Model Specifications](#model-specifications)

---

## Actuarial Framework

### Industry Standards (Regulatory)

#### Premium Calculation

```
Gross Premium = Pure Premium / (1 - Loading Percentage)
```

Where:
- **Pure Premium** = Expected Claims (probability × magnitude of loss)
- **Loading Percentage** = Admin costs + reserves + profit + risk margin

#### Objective Risk (Morrisey)

```
Objective Risk = σ / (μ√N)
```

Risk decreases with more members (law of large numbers). A pool of 10,000 has much lower objective risk than a pool of 100.

#### ACA Requirements

| Requirement | Rule |
|-------------|------|
| MLR (Individual/Small) | ≥80% |
| MLR (Large Group) | ≥85% |
| Age Rating | Max 3:1 ratio |
| Tobacco Surcharge | Max 50% (1.5x) |
| Health Status Rating | **Prohibited** |

### Apollo Parameters (Governance)

| Parameter | Value | Notes |
|-----------|-------|-------|
| **MLR** | **90%+ (REQUIRED)** | Not just a target |
| Admin Load | 8% | Blockchain efficiency |
| Reserve Margin | 2% | Within 10% total loading |
| Total Loading | ≤10% | Required for 90%+ MLR |

**Critical Math**: MLR = 1 - Loading. For 90% MLR, loading must be ≤10%.

### CAR Zone System

Capital Adequacy Ratio (CAR) is Apollo's on-chain solvency metric.

| Zone | CAR Range | Enrollment | ShockFactor Limit |
|------|-----------|------------|-------------------|
| 🟢 Green | ≥150% | Unlimited | 1.0x |
| 🟡 Yellow | 125-149% | Throttled | 1.2x |
| 🟠 Orange | 100-124% | Limited | 1.5x |
| 🔴 Red | <100% | Frozen | 2.0x |

### Three-Tier Reserve Structure

| Tier | Purpose | Target |
|------|---------|--------|
| Tier 0 | Liquidity Buffer | 7-30 days claims |
| Tier 1 | Operating Reserve | 30-60 days claims |
| Tier 2 | Contingent Capital | 6+ months claims |

**Claims Waterfall**: Claims → Tier 0 → Tier 1 → Tier 2 → Staked APH

### Key Constants

```rust
// Industry Standards (regulatory)
pub const ACA_MIN_MLR_SMALL_BPS: u16 = 8000;        // 80%
pub const ACA_MAX_AGE_RATIO: u8 = 3;                 // 3:1
pub const ACA_MAX_TOBACCO_SURCHARGE_BPS: u16 = 5000; // 50%

// Apollo Requirements (governance)
pub const APOLLO_TARGET_MLR_BPS: u16 = 9000;         // 90% REQUIRED
pub const MAX_TOTAL_LOADING_BPS: u16 = 1000;         // 10% max
pub const ADMIN_LOAD_BPS: u16 = 800;                 // 8%
pub const RESERVE_MARGIN_BPS: u16 = 200;             // 2%
pub const TARGET_CAR_BPS: u16 = 12500;               // 125%
```

---

## AI/ML Architecture

Apollo Care is designed from the ground up with AI/ML integration as a first-class citizen.

### Design Principles

1. **Data-First**: Every transaction generates structured data for ML pipelines
2. **Verifiable AI**: All AI decisions recorded on-chain via oracle
3. **Graduated Autonomy**: Rules → Hybrid → Full ML as data accumulates
4. **Human-in-the-Loop**: Committee override for edge cases

### Data Flow Architecture

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│   On-Chain       │────▶│   Event Logs     │────▶│  ML Feature      │
│   Transactions   │     │   (Indexed)      │     │  Pipeline        │
└──────────────────┘     └──────────────────┘     └────────┬─────────┘
                                                          │
                                                          ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        ML MODEL LAYERS                              │
├─────────────────────┬───────────────────┬───────────────────────────┤
│  Claims Triage      │  Fraud Detection  │  Dynamic Pricing          │
│  (XGBoost/NN)       │  (Anomaly Det.)   │  (Gradient Boosting)      │
└─────────────────────┴───────────────────┴───────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    CLAIMS ORACLE PROGRAM                            │
│         (Verifiable AI decisions recorded on-chain)                 │
└─────────────────────────────────────────────────────────────────────┘
```

### Three-Tier Claims with AI

```rust
pub enum ClaimDecisionPath {
    // Tier 1: Fast-Lane (<$500, >95% confidence)
    FastLane {
        ai_confidence: u16,
        fraud_score: u16,
        auto_approved: bool,
    },
    
    // Tier 2: AI-Assisted ($500-$5,000, 85%+ confidence)
    AiReview {
        ai_recommendation: ClaimVerdict,
        confidence: u16,
        fraud_indicators: Vec<FraudFlag>,
        requires_human: bool,
    },
    
    // Tier 3: Committee (>$5,000 or flagged)
    CommitteeReview {
        ai_analysis: ClaimAnalysis,
        committee_votes: Vec<Vote>,
        final_decision: ClaimVerdict,
    },
}
```

### AI Oracle Structure

```rust
#[account]
pub struct AiDecisionRecord {
    pub claim_id: Pubkey,
    pub model_version: [u8; 32],
    pub confidence_score: u16,
    pub fraud_score: u16,
    pub recommendation: AiRecommendation,
    pub inference_timestamp: i64,
    pub feature_hash: [u8; 32],
    pub oracle_authority: Pubkey,
}

pub enum AiRecommendation {
    AutoApprove,
    ApproveWithReview,
    RequiresCommittee,
    Deny { reason_code: u16 },
    FraudAlert,
}
```

---

## Model Specifications

### Claims Triage Model

| Attribute | Value |
|-----------|-------|
| Purpose | Route claims to processing tier |
| Architecture | XGBoost ensemble (500 trees) |
| Output | Tier classification + confidence |
| Latency | <100ms |

**Features**:
- Claim amount (normalized)
- Member tenure, claims history
- Provider patterns, approval rates
- Diagnosis risk score
- Service type, timing

### Fraud Detection Model

| Attribute | Value |
|-----------|-------|
| Purpose | Identify fraudulent claims |
| Architecture | Isolation Forest + GNN + LSTM |
| Output | Fraud probability + flags |

**Fraud Indicators**:
- Unusual claim volume
- Provider pattern anomaly
- Upcoding indicators
- Phantom billing
- Unbundling patterns
- Geographic/temporal anomalies

### Dynamic Pricing Model

| Attribute | Value |
|-----------|-------|
| Purpose | Real-time premium adjustments |
| Architecture | Gradient Boosting + Time Series |
| Output | ShockFactor recommendation |

---

## References

- Morrisey, "Health Insurance" 3rd Edition
- ACA regulations (42 USC §300gg-18)
- NAIC Risk-Based Capital guidelines

---

*Consolidated from: ACTUARIAL_ALIGNMENT.md, AI_OPTIMIZED_ARCHITECTURE.md*
