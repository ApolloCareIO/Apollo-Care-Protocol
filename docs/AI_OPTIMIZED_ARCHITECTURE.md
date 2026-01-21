# Apollo Care Protocol - AI-Optimized Architecture

## Overview

Apollo Care Protocol is designed from the ground up with AI/ML integration as a first-class citizen, not an afterthought. This document describes the architectural decisions that enable machine learning optimization across pricing, claims processing, risk management, and fraud detection.

---

## AI-Optimized Design Principles

### 1. Data-First Architecture

Every transaction, claim, and member interaction generates structured data that feeds ML pipelines:

```
┌───────────────────────────────────────────────────────────────────────────┐
│                         DATA FLOW ARCHITECTURE                            │
├───────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────────────────┐   │
│  │   On-Chain   │────▶│  Event Logs  │────▶│  ML Feature Pipeline     │   │
│  │ Transactions │     │   (Indexed)  │     │  (Off-chain Processing)  │   │
│  └──────────────┘     └──────────────┘     └───────────┬──────────────┘   │
│                                                        │                  │
│                                                        ▼                  │
│  ┌──────────────────────────────────────────────────────────────────────┐ │
│  │                        ML MODEL LAYERS                               │ │
│  ├──────────────────┬───────────────────┬───────────────────────────────┤ │
│  │  Claims Triage   │  Fraud Detection  │  Dynamic Pricing              │ │
│  │  (XGBoost/NN)    │  (Anomaly Det.)   │  (Gradient Boosting)          │ │
│  └──────────────────┴───────────────────┴───────────────────────────────┘ │
│                              │                                            │
│                              ▼                                            │
│  ┌──────────────────────────────────────────────────────────────────────┐ │
│  │                    CLAIMS ORACLE PROGRAM                             │ │
│  │         (Verifiable AI decisions recorded on-chain)                  │ │
│  └──────────────────────────────────────────────────────────────────────┘ │
│                                                                           │
└───────────────────────────────────────────────────────────────────────────┘
```

### 2. Three-Tier Claims Processing with AI

```rust
// Claims processing workflow - AI at each decision point

pub enum ClaimDecisionPath {
    // Tier 1: Fast-Lane Auto-Approval (<$500, high confidence)
    // - ML model: Real-time inference
    // - Latency: <100ms
    // - Confidence threshold: 95%
    FastLane {
        ai_confidence: u16,      // basis points (9500 = 95%)
        fraud_score: u16,        // lower is better
        auto_approved: bool,
    },
    
    // Tier 2: AI-Assisted Review ($500-$5,000)
    // - ML model: Deep analysis with human oversight
    // - Latency: <24 hours
    // - Confidence threshold: 85%
    AiReview {
        ai_recommendation: ClaimVerdict,
        confidence: u16,
        fraud_indicators: Vec<FraudFlag>,
        requires_human: bool,
    },
    
    // Tier 3: Committee Review (>$5,000 or flagged)
    // - ML model: Provides analysis, humans decide
    // - Latency: 3-5 business days
    // - Always requires attestation
    CommitteeReview {
        ai_analysis: ClaimAnalysis,
        committee_votes: Vec<Vote>,
        final_decision: ClaimVerdict,
    },
}
```

### 3. AI Oracle Integration

The `ClaimsOracle` program enables verifiable AI decisions on-chain:

```rust
/// AI decision record stored on-chain for transparency
#[account]
pub struct AiDecisionRecord {
    /// Claim this decision relates to
    pub claim_id: Pubkey,
    
    /// Model version that produced this decision
    pub model_version: [u8; 32],
    
    /// Decision confidence (basis points)
    pub confidence_score: u16,
    
    /// Fraud probability score (0-10000 bps)
    pub fraud_score: u16,
    
    /// Recommended action
    pub recommendation: AiRecommendation,
    
    /// Timestamp of inference
    pub inference_timestamp: i64,
    
    /// Hash of input features (for reproducibility)
    pub feature_hash: [u8; 32],
    
    /// Oracle authority that submitted this decision
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

## ML Model Architecture

### Claims Triage Model

**Purpose**: Route claims to appropriate processing tier

**Architecture**: 
- XGBoost ensemble with 500 trees
- Features: claim amount, member history, provider patterns, diagnosis codes, time of submission
- Output: Tier classification + confidence score

**Training Data**:
- Historical claims with outcomes
- Fraud labels (confirmed/suspected)
- Processing time and complexity metrics

```python
# Feature engineering for claims triage
features = {
    'claim_amount': float,           # Normalized claim amount
    'member_tenure_days': int,       # Days since enrollment
    'member_claims_ytd': int,        # Claims count this year
    'member_claims_total_ytd': float, # Total claims $ this year
    'provider_claim_count': int,     # Provider's claim volume
    'provider_avg_approval_rate': float,
    'diagnosis_risk_score': float,   # ICD-10 based risk
    'service_type': categorical,     # Office visit, ER, surgery, etc.
    'hour_of_submission': int,       # Time patterns
    'day_of_week': int,
    'is_weekend': bool,
}
```

### Fraud Detection Model

**Purpose**: Identify potentially fraudulent claims

**Architecture**:
- Isolation Forest for anomaly detection
- Graph Neural Network for provider-member relationship analysis
- LSTM for temporal pattern detection

**Fraud Indicators**:
```rust
pub enum FraudFlag {
    UnusualClaimVolume,          // Statistical outlier
    ProviderPatternAnomaly,      // Provider behavior change
    UpccodingIndicators,         // Systematic billing inflation
    PhantomBilling,              // Services not rendered
    UnbundlingPattern,           // Splitting procedures
    IdentityRisk,                // Member identity concerns
    GeographicAnomaly,           // Impossible travel patterns
    TemporalAnomaly,             // Unusual timing patterns
}
```

### Dynamic Pricing Model

**Purpose**: Real-time premium adjustments based on pool risk

**Architecture**:
- Gradient Boosting for base rate prediction
- Time series forecasting for claims trends
- Cohort analysis for adverse selection detection

**ShockFactor Calculation**:
```rust
/// AI-driven ShockFactor adjustment
pub fn calculate_shock_factor(
    pool_metrics: &PoolMetrics,
    ai_forecast: &ClaimsForecast,
) -> u16 {
    // Base: 10000 = 1.0x (no adjustment)
    let mut shock_factor: u16 = 10000;
    
    // CAR-based adjustment
    if pool_metrics.car < 12500 {  // Below 125%
        shock_factor += 500;  // +5% per 25% under target
    }
    
    // AI forecast adjustment
    if ai_forecast.next_month_claims > pool_metrics.expected_claims * 110 / 100 {
        let overage = (ai_forecast.next_month_claims - pool_metrics.expected_claims) 
            * 100 / pool_metrics.expected_claims;
        shock_factor += overage as u16 * 10;  // 1% per 10% forecast overage
    }
    
    // Cap at 2.0x (20000 bps)
    shock_factor.min(20000)
}
```

---

## UCR Price Validation

Usual, Customary, and Reasonable (UCR) pricing with AI enhancement:

```rust
#[account]
pub struct UcrPriceEntry {
    /// CPT/HCPCS procedure code
    pub procedure_code: [u8; 8],
    
    /// Geographic region code
    pub region_code: u16,
    
    /// Price percentiles (in USDC cents)
    pub p25_price: u64,
    pub p50_price: u64,
    pub p75_price: u64,
    pub p90_price: u64,
    
    /// AI-predicted fair price for this region
    pub ai_fair_price: u64,
    
    /// Confidence in AI price
    pub ai_confidence: u16,
    
    /// Last update timestamp
    pub last_updated: i64,
    
    /// Data source count (for credibility)
    pub sample_count: u32,
}

/// Validate claim amount against UCR
pub fn validate_claim_price(
    claim_amount: u64,
    ucr: &UcrPriceEntry,
) -> PriceValidation {
    if claim_amount <= ucr.p75_price {
        PriceValidation::Approved
    } else if claim_amount <= ucr.p90_price {
        PriceValidation::RequiresReview { reason: "Above 75th percentile" }
    } else {
        PriceValidation::Flagged { 
            reason: "Above 90th percentile",
            ucr_p90: ucr.p90_price,
            claimed: claim_amount,
        }
    }
}
```

---

## Cohort Analysis for Adverse Selection

AI continuously monitors for adverse selection patterns:

```rust
#[account]
pub struct CohortMetrics {
    /// Cohort identifier (enrollment period)
    pub cohort_id: u32,
    
    /// Enrollment start timestamp
    pub enrollment_start: i64,
    
    /// Number of members in cohort
    pub member_count: u32,
    
    /// Average claims per member
    pub avg_claims_per_member: u64,
    
    /// Loss ratio for this cohort
    pub loss_ratio_bps: u16,
    
    /// AI-predicted future loss ratio
    pub predicted_loss_ratio: u16,
    
    /// Adverse selection risk score (0-100)
    pub adverse_selection_score: u8,
    
    /// Flags
    pub flags: CohortFlags,
}

bitflags! {
    pub struct CohortFlags: u8 {
        const NORMAL = 0b00000000;
        const HIGH_UTILIZATION = 0b00000001;
        const ADVERSE_SELECTION_RISK = 0b00000010;
        const RAPID_ENROLLMENT = 0b00000100;
        const DISENROLLMENT_SPIKE = 0b00001000;
    }
}
```

---

## Fast-Lane Abuse Prevention

AI-powered detection of fast-lane system gaming:

```rust
#[account]
pub struct FastLaneTracker {
    /// Member being tracked
    pub member: Pubkey,
    
    /// Rolling 30-day fast-lane claims
    pub claims_30d: u16,
    
    /// Rolling 90-day fast-lane claims
    pub claims_90d: u16,
    
    /// Total fast-lane amount 30d (USDC)
    pub amount_30d: u64,
    
    /// Pattern analysis flags
    pub pattern_flags: PatternFlags,
    
    /// AI suspicion score (0-100)
    pub suspicion_score: u8,
    
    /// Cooldown until timestamp (if throttled)
    pub cooldown_until: i64,
}

/// Check if member can use fast-lane
pub fn check_fast_lane_eligibility(
    tracker: &FastLaneTracker,
    current_time: i64,
) -> FastLaneEligibility {
    // Cooldown check
    if tracker.cooldown_until > current_time {
        return FastLaneEligibility::Throttled { 
            until: tracker.cooldown_until 
        };
    }
    
    // Frequency check (max 5 fast-lane claims per 30 days)
    if tracker.claims_30d >= 5 {
        return FastLaneEligibility::LimitReached;
    }
    
    // Amount check (max $2,000 fast-lane per 30 days)
    if tracker.amount_30d >= 200_000 {  // cents
        return FastLaneEligibility::AmountLimitReached;
    }
    
    // AI suspicion check
    if tracker.suspicion_score > 75 {
        return FastLaneEligibility::RequiresReview;
    }
    
    FastLaneEligibility::Eligible
}
```

---

## IBNR (Incurred But Not Reported) Estimation

AI-enhanced reserve estimation:

```rust
/// IBNR calculation with AI enhancement
pub fn calculate_ibnr(
    daily_claims_avg: u64,
    reporting_lag_days: u16,
    ai_adjustment_factor: u16,  // basis points (10000 = 1.0x)
) -> u64 {
    // Base IBNR = Average Daily Claims × Reporting Lag × Development Factor
    let base_ibnr = daily_claims_avg
        .checked_mul(reporting_lag_days as u64)
        .unwrap()
        .checked_mul(115)  // 1.15 development factor
        .unwrap()
        .checked_div(100)
        .unwrap();
    
    // AI adjustment (can increase or decrease based on patterns)
    base_ibnr
        .checked_mul(ai_adjustment_factor as u64)
        .unwrap()
        .checked_div(10000)
        .unwrap()
}
```

---

## Model Governance

### Model Registry

All AI models used in production are registered on-chain:

```rust
#[account]
pub struct ModelRegistry {
    /// Model unique identifier
    pub model_id: [u8; 32],
    
    /// Model version (semantic versioning encoded)
    pub version: u32,
    
    /// Model type
    pub model_type: ModelType,
    
    /// IPFS hash of model artifacts
    pub artifact_hash: [u8; 32],
    
    /// Training data hash (for reproducibility)
    pub training_data_hash: [u8; 32],
    
    /// Performance metrics
    pub accuracy: u16,      // basis points
    pub precision: u16,
    pub recall: u16,
    pub f1_score: u16,
    
    /// Approval status
    pub approved_by: Pubkey,  // DAO governance
    pub approved_at: i64,
    
    /// Active status
    pub is_active: bool,
}

pub enum ModelType {
    ClaimsTriage,
    FraudDetection,
    PricingOptimization,
    AdverseSelectionDetection,
    IbnrEstimation,
}
```

### Model Update Process

1. **Proposal**: Data science team proposes new model version
2. **Validation**: Independent validation on held-out test set
3. **Shadow Mode**: Run alongside production for 2 weeks
4. **Committee Review**: Actuarial committee reviews performance
5. **DAO Vote**: Community approves model deployment
6. **Gradual Rollout**: 10% → 50% → 100% traffic

---

## Performance Targets

| Metric | Target | Current |
|--------|--------|---------|
| Fast-lane auto-approval rate | >80% of eligible claims | - |
| Fraud detection precision | >95% | - |
| Fraud detection recall | >90% | - |
| Claims processing time (fast-lane) | <100ms | - |
| Claims processing time (AI review) | <24 hours | - |
| False positive rate (fraud) | <2% | - |
| Model inference latency | <50ms | - |

---

## Security Considerations

### Oracle Security

- Multiple oracle providers for redundancy
- Stake-weighted consensus for AI decisions
- Slashing for incorrect or malicious oracles
- Rate limiting on oracle submissions

### Model Security

- Adversarial robustness testing
- Input validation and sanitization
- Model drift monitoring
- Regular retraining with fresh data

### Privacy

- No PHI in on-chain data
- Federated learning for model updates
- Differential privacy for aggregate statistics
- HIPAA-compliant off-chain processing

---

## Future Enhancements

### Phase 2
- Reinforcement learning for optimal reserve management
- Natural language processing for claim document analysis
- Computer vision for medical bill OCR
- Personalized pricing recommendations

### Phase 3
- Federated learning across partner networks
- On-chain ML inference (as Solana compute improves)
- Cross-chain oracle aggregation
- Predictive member health analytics (opt-in)

---

## References

1. Morrisey, M.A. (2020). *Health Insurance*, Third Edition. Health Administration Press.
2. Apollo Care Protocol Actuarial Specification
3. Solana Program Library documentation
4. XGBoost: A Scalable Tree Boosting System (Chen & Guestrin, 2016)
5. Isolation Forest for Anomaly Detection (Liu, Ting & Zhou, 2008)
