/**
 * Apollo Care Protocol - Protocol Constants
 */

/** Basis points denominator (10000 = 100%) */
export const BPS = 10000;

/** Target Medical Loss Ratio (90%+) */
export const TARGET_MLR_BPS = 9000;

/** ACA minimum MLR requirements */
export const ACA_MLR = {
  SMALL_GROUP: 8000,  // 80% minimum
  LARGE_GROUP: 8500,  // 85% minimum
} as const;

/** Loading breakdown (10% total = 90% MLR) */
export const LOADING = {
  ADMIN: 800,         // 8% - claims processing, AI, support
  RESERVE_MARGIN: 200, // 2% - builds surplus
  TOTAL: 1000,        // 10% total loading
} as const;

/** CAR Zone thresholds (basis points) */
export const CAR_ZONES = {
  GREEN: 15000,   // ≥150% - Unlimited enrollment
  YELLOW: 12500,  // ≥125% to <150% - Max 500/month
  ORANGE: 10000,  // ≥100% to <125% - Max 100/month
  // RED: <100% (below ORANGE threshold) - Enrollment frozen
} as const;

/** Reserve tier targets (days of coverage) */
export const RESERVE_DAYS = {
  TIER0_MIN: 0,   // Liquidity buffer minimum
  TIER0_TARGET: 30, // Liquidity buffer target
  TIER1_MIN: 30,   // Operating reserve minimum
  TIER1_TARGET: 60, // Operating reserve target (+ IBNR)
  TIER2_MIN: 180,  // Contingent capital minimum
  TIER2_TARGET: 365, // Contingent capital target (6+ months)
} as const;

/** Staking lock periods (days) */
export const STAKING_LOCKS = {
  CONSERVATIVE: 30,   // 3-5% APY, 2% max loss
  STANDARD: 90,       // 6-8% APY, 5% max loss
  AGGRESSIVE: 180,    // 10-15% APY, 10% max loss
} as const;

/** Fast-lane auto-approval limits */
export const FAST_LANE = {
  MAX_AMOUNT: 500_000000,      // $500 USDC
  MAX_PER_30_DAYS: 5,          // Max claims
  MAX_TOTAL_30_DAYS: 2000_000000, // $2,000 USDC
  MIN_AI_CONFIDENCE: 9500,     // 95%
  MAX_FRAUD_SCORE: 1000,       // 10%
} as const;

/** Reinsurance defaults (bootstrap) */
export const REINSURANCE = {
  SPECIFIC_ATTACHMENT: 50000_000000,  // $50k
  SPECIFIC_COVERAGE_BPS: 9000,        // 90%
  AGGREGATE_TRIGGER_BPS: 11000,       // 110%
  AGGREGATE_COVERAGE_BPS: 10000,      // 100%
  AGGREGATE_CAP_BPS: 15000,           // 150%
} as const;

/** Token allocations (bps of 1B total supply) */
export const TOKEN_ALLOCATIONS = {
  COMMUNITY_ECOSYSTEM: 4700,  // 47% = 470M APH
  CORE_TEAM: 2200,            // 22% = 220M APH
  SEED_STRATEGIC: 1000,       // 10% = 100M APH
  INSURANCE_RESERVE: 1000,    // 10% = 100M APH
  LIQUIDITY_EXCHANGES: 600,   // 6% = 60M APH
  OPERATIONS: 500,            // 5% = 50M APH
} as const;

/** Shock factor range */
export const SHOCK_FACTOR = {
  MIN: 10000,  // 1.0x
  MAX: 20000,  // 2.0x
  DEFAULT: 10000,
} as const;

/** IBNR calculation parameters */
export const IBNR = {
  DEVELOPMENT_FACTOR: 11500,  // 1.15x
  DEFAULT_LAG_DAYS: 21,       // Average reporting lag
} as const;
