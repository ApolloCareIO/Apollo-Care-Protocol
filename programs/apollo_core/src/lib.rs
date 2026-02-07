// programs/apollo_core/src/lib.rs
//
// Apollo Core - Shared Constants and Token-2022 Utilities
// ========================================================
//
// This module provides:
// - APH Token-2022 mint address and configuration
// - Token allocation constants per tokenomics
// - Transfer fee handling utilities
// - Cross-program shared types
// - Protocol phase tracking (Phase 1/2/3 transitions)
//
// APH Token: 6S3T6f1mmhxQWKR1gMiZR1SZLpu396jnnRMGqAZUj3Qj (Token-2022)
// Authority: 7jMC5SKBDmweVLst8YKTNP77wX7RpUpV6MvYVcfqswyt

use anchor_lang::prelude::*;

declare_id!("DHGQUHAXRvEiHA4J3JhxKdSLkqvyKPyZYaciLfMA5yok");

// =============================================================================
// SUBMODULES
// =============================================================================

/// Protocol phase tracking (Phase 1→2→3 transitions)
pub mod phase;

// Re-export phase types for convenience
pub use phase::{
    Phase1To2Requirements, Phase2To3Requirements, PhaseComplianceFlags, ProtocolPhase,
    ProtocolPhaseState,
};

// =============================================================================
// APH TOKEN-2022 CONFIGURATION
// =============================================================================

/// APH Token Mint Address (Token-2022 program)
/// Production: 6S3T6f1mmhxQWKR1gMiZR1SZLpu396jnnRMGqAZUj3Qj
pub mod aph_token {
    use super::*;

    /// APH Token-2022 Mint Address (mainnet)
    pub const APH_MINT: &str = "6S3T6f1mmhxQWKR1gMiZR1SZLpu396jnnRMGqAZUj3Qj";

    /// Mint Authority (Christian's wallet)
    pub const MINT_AUTHORITY: &str = "7jMC5SKBDmweVLst8YKTNP77wX7RpUpV6MvYVcfqswyt";

    /// Token decimals (standard 9 for Solana tokens)
    pub const DECIMALS: u8 = 9;

    /// Total supply: 1 billion APH (1_000_000_000 * 10^9)
    pub const TOTAL_SUPPLY: u64 = 1_000_000_000_000_000_000; // 1B with 9 decimals

    /// Transfer fee basis points (2% = 200 bps)
    /// APH was minted as Token-2022 from day one with transfer fee extension
    /// Fee rate is 0% during presale, activated to 2% post-presale via config authority
    pub const TRANSFER_FEE_BPS: u16 = 200;

    /// Maximum transfer fee per transaction
    /// Calculated as 0.02% of total supply = 200,000 APH
    /// This caps large transfers to prevent excessive fees
    /// Formula: 1,000,000,000 * 0.0002 = 200,000 APH
    pub const MAX_TRANSFER_FEE: u64 = 200_000_000_000_000; // 200,000 APH (with 9 decimals)

    /// Helper to get APH mint pubkey
    pub fn mint_pubkey() -> Pubkey {
        APH_MINT.parse().expect("Invalid APH mint address")
    }

    /// Helper to get mint authority pubkey
    pub fn mint_authority_pubkey() -> Pubkey {
        MINT_AUTHORITY
            .parse()
            .expect("Invalid mint authority address")
    }
}

// =============================================================================
// TOKEN ALLOCATIONS (Per Official Tokenomics)
// =============================================================================
//
// Apollo Care tokenizes the healthcare coverage infrastructure layer.
// $APH is NOT primarily a speculative asset - it serves three core functions:
//
// 1. GOVERNANCE: Members vote on coverage policies, pricing, committee elections
// 2. CAPITAL STAKING: APH provides decentralized Tier-2 backstop capital
// 3. COMMUNITY INCENTIVES: Rewards for participation, referrals, wellness
//
// The token model ensures the community owns the infrastructure, not
// for-profit shareholders. This is healthcare as a public utility.

/// Token allocation percentages and amounts
/// Total: 1,000,000,000 APH (1 billion) - FIXED SUPPLY, NO INFLATION
pub mod allocations {
    use super::*;

    // ===== OFFICIAL ALLOCATION PERCENTAGES =====

    /// Community & Ecosystem Fund: 47% (470M APH)
    /// Purpose: The LARGEST allocation - underscores community-first ethos
    /// Includes: ICO sales, member rewards, governance incentives, developer grants,
    /// partnership programs, marketing. Distributed via DAO governance over time.
    /// This is NOT a single ICO bucket - it's the community's treasury.
    pub const COMMUNITY_ECOSYSTEM_BPS: u16 = 4700;
    pub const COMMUNITY_ECOSYSTEM_AMOUNT: u64 = 470_000_000_000_000_000; // 470M APH

    /// Core Team & Advisors: 22% (220M APH)
    /// Purpose: Founding team and key advisors who build the platform
    /// Vesting: Multi-year schedule (typically 4 years with 1-year cliff)
    /// Ensures long-term alignment - team value grows only if Apollo succeeds
    pub const CORE_TEAM_BPS: u16 = 2200;
    pub const CORE_TEAM_AMOUNT: u64 = 220_000_000_000_000_000; // 220M APH

    /// Seed & Strategic Investors: 10% (100M APH)
    /// Purpose: Early backers who provided initial funding and strategic support
    /// Note: Modest allocation reflects Apollo's ethos of community over big investors
    /// Vesting: Locked and released gradually per milestone/time-based schedule
    pub const SEED_INVESTORS_BPS: u16 = 1000;
    pub const SEED_INVESTORS_AMOUNT: u64 = 100_000_000_000_000_000; // 100M APH

    /// Insurance Reserve: 10% (100M APH)
    /// Purpose: DAO-controlled emergency fund backing the coverage pool
    /// Function: Can be staked to bolster capital, or liquidated via governance
    /// vote to raise USDC during severe claim events (black swan scenarios)
    /// This is the protocol's "rainy day" APH stash - converts to capital if needed
    pub const INSURANCE_RESERVE_BPS: u16 = 1000;
    pub const INSURANCE_RESERVE_AMOUNT: u64 = 100_000_000_000_000_000; // 100M APH

    /// Liquidity & Exchanges: 6% (60M APH)
    /// Purpose: DEX liquidity pools (Raydium, etc.) and exchange listings
    /// Critical for: Price stability, minimal slippage, market depth
    /// Risk management: Deep liquidity allows emergency token sales without
    /// destabilizing price during claims spikes
    pub const LIQUIDITY_EXCHANGES_BPS: u16 = 600;
    pub const LIQUIDITY_EXCHANGES_AMOUNT: u64 = 60_000_000_000_000_000; // 60M APH

    /// Operations: 5% (50M APH)
    /// Purpose: Platform maintenance, infrastructure, legal/regulatory, overhead
    /// Goal: Lean operations to maximize Medical Loss Ratio (90%+ MLR target)
    /// Controlled by DAO with transparent governance approvals
    pub const OPERATIONS_BPS: u16 = 500;
    pub const OPERATIONS_AMOUNT: u64 = 50_000_000_000_000_000; // 50M APH

    // ===== VESTING SCHEDULES =====

    /// Team vesting: 4 years with 1-year cliff (industry standard)
    pub const TEAM_CLIFF_SECONDS: i64 = 365 * 24 * 60 * 60; // 1 year
    pub const TEAM_VESTING_SECONDS: i64 = 4 * 365 * 24 * 60 * 60; // 4 years

    /// Seed investor vesting: 12-24+ months, milestone-based
    pub const SEED_CLIFF_SECONDS: i64 = 6 * 30 * 24 * 60 * 60; // 6 months
    pub const SEED_VESTING_SECONDS: i64 = 2 * 365 * 24 * 60 * 60; // 2 years

    /// Allocation category enumeration
    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
    #[repr(u8)]
    pub enum AllocationCategory {
        /// Community & Ecosystem - 47% - DAO treasury for growth
        CommunityEcosystem = 0,
        /// Core Team - 22% - Builders with vesting
        CoreTeam = 1,
        /// Seed Investors - 10% - Early backers with vesting
        SeedInvestors = 2,
        /// Insurance Reserve - 10% - Emergency APH fund
        InsuranceReserve = 3,
        /// Liquidity - 6% - DEX pools
        Liquidity = 4,
        /// Operations - 5% - Platform costs
        Operations = 5,
    }

    impl AllocationCategory {
        pub fn amount(&self) -> u64 {
            match self {
                AllocationCategory::CommunityEcosystem => COMMUNITY_ECOSYSTEM_AMOUNT,
                AllocationCategory::CoreTeam => CORE_TEAM_AMOUNT,
                AllocationCategory::SeedInvestors => SEED_INVESTORS_AMOUNT,
                AllocationCategory::InsuranceReserve => INSURANCE_RESERVE_AMOUNT,
                AllocationCategory::Liquidity => LIQUIDITY_EXCHANGES_AMOUNT,
                AllocationCategory::Operations => OPERATIONS_AMOUNT,
            }
        }

        pub fn bps(&self) -> u16 {
            match self {
                AllocationCategory::CommunityEcosystem => COMMUNITY_ECOSYSTEM_BPS,
                AllocationCategory::CoreTeam => CORE_TEAM_BPS,
                AllocationCategory::SeedInvestors => SEED_INVESTORS_BPS,
                AllocationCategory::InsuranceReserve => INSURANCE_RESERVE_BPS,
                AllocationCategory::Liquidity => LIQUIDITY_EXCHANGES_BPS,
                AllocationCategory::Operations => OPERATIONS_BPS,
            }
        }

        pub fn is_vested(&self) -> bool {
            matches!(
                self,
                AllocationCategory::CoreTeam | AllocationCategory::SeedInvestors
            )
        }

        /// Returns true if this allocation is controlled by DAO governance
        pub fn is_dao_controlled(&self) -> bool {
            matches!(
                self,
                AllocationCategory::CommunityEcosystem
                    | AllocationCategory::InsuranceReserve
                    | AllocationCategory::Operations
            )
        }
    }

    /// Verify allocations sum to 100%
    pub fn verify_allocations() -> bool {
        let total_bps = COMMUNITY_ECOSYSTEM_BPS
            + CORE_TEAM_BPS
            + SEED_INVESTORS_BPS
            + INSURANCE_RESERVE_BPS
            + LIQUIDITY_EXCHANGES_BPS
            + OPERATIONS_BPS;
        total_bps == 10000 // 100%
    }
}

// =============================================================================
// TOKEN-2022 UTILITIES
// =============================================================================

pub mod token_utils {
    use super::*;

    /// Calculate transfer amount accounting for potential transfer fees
    /// Returns (amount_to_send, expected_fee)
    ///
    /// Note: Transfer fee is not yet enabled but this function is designed
    /// to work correctly when it is enabled post-presale.
    pub fn calculate_transfer_with_fee(
        amount: u64,
        transfer_fee_bps: u16,
        max_fee: u64,
    ) -> (u64, u64) {
        if transfer_fee_bps == 0 {
            return (amount, 0);
        }

        // Calculate fee: amount * fee_bps / 10000
        let fee = amount
            .saturating_mul(transfer_fee_bps as u64)
            .checked_div(10000)
            .unwrap_or(0);

        // Cap at max fee
        let actual_fee = fee.min(max_fee);

        (amount, actual_fee)
    }

    /// Calculate the gross amount needed to transfer a net amount after fees
    /// If you want the recipient to receive `net_amount`, how much do you send?
    pub fn calculate_gross_for_net(net_amount: u64, transfer_fee_bps: u16, max_fee: u64) -> u64 {
        if transfer_fee_bps == 0 {
            return net_amount;
        }

        // gross = net * 10000 / (10000 - fee_bps)
        let denominator = 10000_u64.saturating_sub(transfer_fee_bps as u64);
        let gross = net_amount
            .saturating_mul(10000)
            .checked_div(denominator)
            .unwrap_or(net_amount);

        // Ensure fee doesn't exceed max
        let fee = gross.saturating_sub(net_amount);
        if fee > max_fee {
            net_amount.saturating_add(max_fee)
        } else {
            gross
        }
    }

    /// Validate that a mint is the APH Token-2022 mint
    pub fn validate_aph_mint(mint_key: &Pubkey) -> bool {
        *mint_key == aph_token::mint_pubkey()
    }

    /// Check if transfer fee extension is likely enabled
    /// This is a heuristic based on account size
    pub fn has_transfer_fee_extension(mint_data_len: usize) -> bool {
        // Token-2022 mint with transfer fee extension is larger than base mint
        // Base mint: 82 bytes
        // With transfer fee: ~200+ bytes depending on extensions
        mint_data_len > 100
    }
}

// =============================================================================
// SHARED ACCOUNT STRUCTURES
// =============================================================================

/// APH Token-2022 Vault configuration for programs
#[account]
#[derive(InitSpace)]
pub struct AphTokenConfig {
    /// The APH Token-2022 mint
    pub aph_mint: Pubkey,

    /// Authority that can manage this config
    pub authority: Pubkey,

    /// Whether transfer fee handling is enabled
    /// Should be true post-presale when 2% fee is active
    pub transfer_fee_active: bool,

    /// Current transfer fee in basis points
    pub transfer_fee_bps: u16,

    /// Maximum fee per transfer
    pub max_transfer_fee: u64,

    /// Timestamp when fee was last updated
    pub fee_updated_at: i64,

    /// Bump for PDA
    pub bump: u8,

    /// Reserved for future use
    #[max_len(32)]
    pub reserved: Vec<u8>,
}

impl AphTokenConfig {
    pub const SEED_PREFIX: &'static [u8] = b"aph_token_config";

    pub fn calculate_fee(&self, amount: u64) -> u64 {
        if !self.transfer_fee_active || self.transfer_fee_bps == 0 {
            return 0;
        }

        let fee = amount
            .saturating_mul(self.transfer_fee_bps as u64)
            .checked_div(10000)
            .unwrap_or(0);

        fee.min(self.max_transfer_fee)
    }
}

// =============================================================================
// VESTING SCHEDULE STRUCTURES
// =============================================================================

/// Vesting schedule for team and seed allocations
#[account]
#[derive(InitSpace)]
pub struct VestingSchedule {
    /// Beneficiary of vesting
    pub beneficiary: Pubkey,

    /// Allocation category (Team or Seed)
    pub category: u8,

    /// Total amount allocated
    pub total_amount: u64,

    /// Amount already claimed
    pub claimed_amount: u64,

    /// Vesting start timestamp
    pub start_time: i64,

    /// Cliff end timestamp (no tokens before this)
    pub cliff_end: i64,

    /// Full vesting end timestamp
    pub vesting_end: i64,

    /// Whether vesting was revoked
    pub is_revoked: bool,

    /// Bump for PDA
    pub bump: u8,
}

impl VestingSchedule {
    pub const SEED_PREFIX: &'static [u8] = b"vesting";

    /// Calculate currently vested amount
    pub fn vested_amount(&self, current_time: i64) -> u64 {
        if self.is_revoked {
            return self.claimed_amount;
        }

        if current_time < self.cliff_end {
            return 0;
        }

        if current_time >= self.vesting_end {
            return self.total_amount;
        }

        // Linear vesting after cliff
        let total_vesting_duration = self.vesting_end - self.start_time;
        let elapsed = current_time - self.start_time;

        self.total_amount
            .saturating_mul(elapsed as u64)
            .checked_div(total_vesting_duration as u64)
            .unwrap_or(0)
    }

    /// Calculate claimable amount (vested - already claimed)
    pub fn claimable_amount(&self, current_time: i64) -> u64 {
        self.vested_amount(current_time)
            .saturating_sub(self.claimed_amount)
    }
}

// =============================================================================
// ACTUARIAL FRAMEWORK & PROTOCOL CONSTANTS
// =============================================================================
//
// Reference: Morrisey, "Health Insurance" 3rd Edition (authoritative textbook)
//
// CORE ACTUARIAL FORMULA (per Morrisey Ch. 6):
//   Gross Premium = Pure Premium / (1 - Loading Percentage)
//   where:
//     - Pure Premium = Expected Claims (probability × magnitude)
//     - Loading = admin costs + reserves + profit + objective risk margin
//
// OBJECTIVE RISK FORMULA (per Morrisey):
//   Objective Risk = σ / (μ√N)
//   where σ = claims dispersion, μ = expected loss, N = covered lives
//   Key insight: Risk DECREASES with more members (law of large numbers)
//
// MLR REQUIREMENTS (ACA, per Morrisey):
//   - Small group/individual: minimum 80% MLR
//   - Large group: minimum 85% MLR
//   - MLR = Claims / Premiums; Loading = 1 - MLR
//
// LOADING PERCENTAGES (Morrisey empirical data):
//   - Large groups (>10,000): ~4%
//   - Mid-size (100-10,000): ~15%
//   - Small groups (<100): ~34%
//   - Individual market: ~50%
//
// Apollo targets 90%+ MLR (10% loading) via blockchain automation,
// which would be exceptional efficiency vs. industry norms.

pub mod actuarial {
    use super::*;

    // =========================================================================
    // SECTION A: INDUSTRY STANDARDS (per Morrisey textbook)
    // These are regulatory requirements or empirical norms, NOT configurable
    // =========================================================================

    /// ACA minimum MLR for small group/individual (80%)
    pub const ACA_MIN_MLR_SMALL_BPS: u16 = 8000;

    /// ACA minimum MLR for large group (85%)
    pub const ACA_MIN_MLR_LARGE_BPS: u16 = 8500;

    /// ACA maximum age rating ratio (3:1 oldest to youngest adult)
    /// Per Morrisey: "capping the oldest-to-youngest adult ratio at 3:1"
    pub const ACA_MAX_AGE_RATIO: u8 = 3;

    /// ACA maximum tobacco surcharge (50% = 1.5x factor)
    /// Per Morrisey: "tobacco surcharges at 50%"
    pub const ACA_MAX_TOBACCO_SURCHARGE_BPS: u16 = 5000; // 50% max increase

    /// Typical loading for individual market per Morrisey (~50%)
    pub const INDUSTRY_LOADING_INDIVIDUAL_BPS: u16 = 5000;

    /// Typical loading for small group per Morrisey (~34%)
    pub const INDUSTRY_LOADING_SMALL_GROUP_BPS: u16 = 3400;

    /// Typical loading for large group per Morrisey (~15%)
    pub const INDUSTRY_LOADING_LARGE_GROUP_BPS: u16 = 1500;

    // =========================================================================
    // SECTION B: APOLLO GOVERNANCE PARAMETERS (configurable by DAO)
    // These are Apollo-specific targets, not actuarial standards
    // CRITICAL: Apollo REQUIRES 90%+ MLR - this means max 10% total loading
    // =========================================================================

    /// Apollo's REQUIRED MLR (90%+ = industry-leading efficiency)
    /// This is a hard requirement, not just a target
    /// MLR = Claims / Premiums; Loading = 1 - MLR
    /// 90% MLR means max 10% loading (admin + reserves combined)
    pub const APOLLO_TARGET_MLR_BPS: u16 = 9000;

    /// Apollo's target admin load (8%)
    /// Aggressive vs. industry (15-50%) but achievable via automation
    /// Note: Must leave room for reserve margin within 10% total loading
    pub const APOLLO_TARGET_ADMIN_LOAD_BPS: u16 = 800;

    /// Apollo's target reserve margin (2%)
    /// Contributes to building surplus beyond expected claims
    /// Note: Admin (8%) + Reserve (2%) = 10% loading = 90% MLR ✓
    pub const APOLLO_TARGET_RESERVE_MARGIN_BPS: u16 = 200;

    // =========================================================================
    // SECTION C: RESERVE TIER STRUCTURE (Apollo innovation)
    // Note: Tiered reserves are Apollo's design, not standard insurance practice
    // Traditional insurers hold unitary reserves + purchase reinsurance
    // =========================================================================

    /// Tier 0: Liquidity Buffer (Apollo design)
    /// Purpose: Real-time claims fund for instant payouts
    /// Typical: ~1 week to 1 month of claims kept liquid
    pub const TIER0_MIN_DAYS: u8 = 0;
    pub const TIER0_TARGET_DAYS: u8 = 30;

    /// Tier 1: Operating Reserve (Apollo design)
    /// Purpose: Absorbs normal claim volatility
    /// Target: 1-2 months of expected claims
    pub const TIER1_MIN_DAYS: u8 = 30;
    pub const TIER1_TARGET_DAYS: u8 = 60;

    /// Tier 2: Contingent Capital (Apollo design)
    /// Purpose: Emergency backstop (DAO Treasury + Staked APH)
    /// Target: 6+ months coverage
    pub const TIER2_MIN_MONTHS: u8 = 6;

    // =========================================================================
    // SECTION D: RESERVE FORMULA (governance-configurable coefficients)
    // Formula: RequiredReserve = k1 × AvgClaims_N + k2 × σ_M + IBNR
    // =========================================================================

    /// k1: Claims average multiplier (governance parameter)
    pub const DEFAULT_RESERVE_K1: u64 = 3_000; // 3.0 (÷1000)

    /// k2: Volatility multiplier (governance parameter)
    pub const DEFAULT_RESERVE_K2: u64 = 1_000; // 1.0 (÷1000)

    /// Lookback period for claims average
    pub const CLAIMS_AVG_LOOKBACK_MONTHS: u8 = 6;

    /// Lookback period for volatility calculation
    pub const VOLATILITY_LOOKBACK_MONTHS: u8 = 12;

    // =========================================================================
    // SECTION E: IBNR (Incurred But Not Reported)
    // Standard actuarial concept - claims occurred but not yet submitted
    // =========================================================================

    /// Average claim reporting lag (governance parameter)
    /// 21 days per actuarial specification
    pub const DEFAULT_IBNR_LAG_DAYS: u8 = 21;

    /// IBNR development factor (governance parameter)
    /// 1.15 = 15% margin for claim development
    pub const DEFAULT_IBNR_DEVELOPMENT_BPS: u16 = 11500;

    // =========================================================================
    // SECTION F: CAPITAL ADEQUACY RATIO (CAR)
    // Apollo governance metric - NOT a standard insurance term
    // Traditional insurers use Risk-Based Capital (RBC) requirements
    // CAR is Apollo's simplified on-chain equivalent
    // =========================================================================

    /// Basis points denominator for calculations
    pub const BPS_DENOMINATOR: u64 = 10000;

    /// Critical CAR - below this, solvency is at risk (100%)
    /// This is the floor where assets = liabilities (no margin)
    pub const CRITICAL_CAR_BPS: u16 = 10000;

    /// Minimum CAR - no safety margin (100%)  
    pub const MIN_CAR_BPS: u16 = 10000;

    /// Target CAR - Apollo's governance target (125%)
    /// Note: This is a DAO-configurable parameter, not an actuarial standard
    pub const TARGET_CAR_BPS: u16 = 12500;

    /// Green Zone CAR threshold (150%+)
    /// Note: Zone thresholds are governance parameters
    pub const GREEN_ZONE_CAR_BPS: u16 = 15000;

    /// CAR Zone definitions for enrollment control
    /// These zones are Apollo's governance framework for managing growth
    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
    #[repr(u8)]
    pub enum CarZone {
        /// Green (CAR ≥ 150%): Unlimited enrollment
        Green = 0,
        /// Yellow (125% ≤ CAR < 150%): Enrollment throttled
        Yellow = 1,
        /// Orange (100% ≤ CAR < 125%): Enhanced underwriting required
        Orange = 2,
        /// Red (CAR < 100%): Enrollment freeze, emergency measures
        Red = 3,
    }

    impl CarZone {
        pub fn from_car_bps(car_bps: u16) -> Self {
            if car_bps >= GREEN_ZONE_CAR_BPS {
                CarZone::Green
            } else if car_bps >= TARGET_CAR_BPS {
                CarZone::Yellow
            } else if car_bps >= MIN_CAR_BPS {
                CarZone::Orange
            } else {
                CarZone::Red
            }
        }

        /// Maximum enrollments per month for this zone (governance parameter)
        pub fn max_monthly_enrollments(&self) -> Option<u32> {
            match self {
                CarZone::Green => None, // Unlimited
                CarZone::Yellow => Some(500),
                CarZone::Orange => Some(100),
                CarZone::Red => Some(0), // Frozen
            }
        }

        /// Whether enhanced underwriting is required
        pub fn requires_enhanced_underwriting(&self) -> bool {
            matches!(self, CarZone::Orange | CarZone::Red)
        }
    }

    // =========================================================================
    // SECTION G: SHOCKFACTOR (Apollo's Special Assessment Mechanism)
    // In mutual insurance, this is called a "special assessment"
    // Apollo calls it ShockFactor - a multiplier on premiums during stress
    // These limits are GOVERNANCE PARAMETERS, not actuarial standards
    // =========================================================================

    /// Base ShockFactor - normal operations (1.0x = no adjustment)
    pub const SHOCK_FACTOR_BASE_BPS: u16 = 10000;

    /// Maximum ShockFactor in Yellow zone (governance parameter)
    /// Actuarial Committee can apply up to 1.2x automatically
    pub const SHOCK_FACTOR_MAX_YELLOW_BPS: u16 = 12000;

    /// Maximum ShockFactor in Orange zone (governance parameter)
    /// Requires Risk Committee approval for 1.2x - 1.5x
    pub const SHOCK_FACTOR_MAX_ORANGE_BPS: u16 = 15000;

    /// Maximum ShockFactor in Red zone (governance parameter)
    /// Requires DAO emergency vote for 1.5x - 2.0x
    /// Requires DAO emergency vote
    pub const SHOCK_FACTOR_MAX_RED_BPS: u16 = 20000;

    // =========================================================================
    // SECTION H: MEDICAL LOSS RATIO (MLR)
    // Per Morrisey: MLR = Claims / Premiums; Loading = 1 - MLR
    // ACA requirements are REGULATORY STANDARDS (not configurable)
    // Apollo targets are GOVERNANCE PARAMETERS
    // =========================================================================

    /// ACA minimum MLR - individual/small group (80%) - REGULATORY
    /// Reference: Morrisey Ch. 6 "Medical Loss Ratios in the ACA"
    pub const ACA_MIN_MLR_INDIVIDUAL_BPS: u16 = 8000;

    /// ACA minimum MLR - large group (85%) - REGULATORY
    pub const ACA_MIN_MLR_LARGE_GROUP_BPS: u16 = 8500;

    /// Apollo's target MLR (90%+) - GOVERNANCE PARAMETER
    /// This is a HARD REQUIREMENT, not just a target
    /// Implies max 10% total loading (admin + reserves combined)
    pub const TARGET_MLR_BPS: u16 = 9000;

    /// Apollo's minimum acceptable MLR (90%) - GOVERNANCE PARAMETER
    /// Apollo Care REQUIRES 90%+ MLR - this is a key differentiator
    pub const MIN_MLR_BPS: u16 = 9000;

    /// Maximum total loading (10%) - GOVERNANCE PARAMETER
    /// This is the sum of admin + reserve margin
    /// Required to achieve 90%+ MLR (loading = 1 - MLR)
    pub const MAX_TOTAL_LOADING_BPS: u16 = 1000;

    /// Administrative load budget (8%) - GOVERNANCE PARAMETER
    /// Per Morrisey: Industry ranges from 4% (large) to 50% (individual)
    /// Apollo targets efficiency via blockchain automation
    pub const ADMIN_LOAD_BPS: u16 = 800;

    /// Reserve margin from premiums (2%) - GOVERNANCE PARAMETER
    /// Contributes to building reserves beyond expected claims
    /// Note: Admin (8%) + Reserve (2%) = 10% loading = 90% MLR
    pub const RESERVE_MARGIN_BPS: u16 = 200;

    // =========================================================================
    // SECTION I: STAKING RISK PARAMETERS (Apollo innovation)
    // These are NOT from standard insurance practice
    // Apollo's mechanism for community capital provision
    // =========================================================================

    /// Conservative tier: 3-5% APY, 2% max loss exposure
    pub const STAKING_CONSERVATIVE_APY_MIN_BPS: u16 = 300;
    pub const STAKING_CONSERVATIVE_APY_MAX_BPS: u16 = 500;
    pub const STAKING_CONSERVATIVE_LOSS_MAX_BPS: u16 = 200;

    /// Standard tier: 6-8% APY, 5% max loss exposure
    pub const STAKING_STANDARD_APY_MIN_BPS: u16 = 600;
    pub const STAKING_STANDARD_APY_MAX_BPS: u16 = 800;
    pub const STAKING_STANDARD_LOSS_MAX_BPS: u16 = 500;

    /// Aggressive tier: 10-15% APY, 10% max loss exposure
    pub const STAKING_AGGRESSIVE_APY_MIN_BPS: u16 = 1000;
    pub const STAKING_AGGRESSIVE_APY_MAX_BPS: u16 = 1500;
    pub const STAKING_AGGRESSIVE_LOSS_MAX_BPS: u16 = 1000;

    // =========================================================================
    // SECTION J: REINSURANCE PARAMETERS
    // Standard insurance practice - stop-loss coverage for catastrophic claims
    // Attachment points are governance parameters, structure is standard
    // =========================================================================

    /// Specific stop-loss attachment point ($100k)
    /// Note: Lower than typical ($150k+) for startup phase variance reduction
    pub const SPECIFIC_STOP_LOSS_ATTACHMENT_USDC: u64 = 100_000_000_000; // $100k

    /// Aggregate stop-loss trigger (110% of expected claims)
    pub const AGGREGATE_STOP_LOSS_TRIGGER_BPS: u16 = 11000;

    /// Aggregate stop-loss ceiling (150% of expected claims)
    pub const AGGREGATE_STOP_LOSS_CEILING_BPS: u16 = 15000;

    // =========================================================================
    // SECTION K: APH LIQUIDATION PARAMETERS (Apollo innovation)
    // When Tier 2 is tapped in emergencies, staked APH may be sold
    // These are governance parameters with circuit breakers
    // =========================================================================

    /// TWAP liquidation window (24-72 hours)
    pub const LIQUIDATION_TWAP_MIN_HOURS: u8 = 24;
    pub const LIQUIDATION_TWAP_MAX_HOURS: u8 = 72;

    /// Circuit breaker: max slippage before halting (15%)
    pub const LIQUIDATION_CIRCUIT_BREAKER_BPS: u16 = 1500;

    /// Max % of staked APH that can be liquidated per month (10%)
    pub const LIQUIDATION_MONTHLY_CAP_BPS: u16 = 1000;

    // =========================================================================
    // SECTION L: ADVERSE SELECTION MITIGATIONS
    // Standard insurance practice adapted for Apollo
    // Per Morrisey: "Insurers deal with adverse selection through
    // the underwriting and rate-making process"
    // =========================================================================

    /// Waiting period for new members (30 days)
    /// Prevents immediate claims after enrollment
    pub const WAITING_PERIOD_DAYS: u8 = 30;

    /// Persistency discount eligibility (12+ months membership)
    pub const PERSISTENCY_DISCOUNT_MIN_MONTHS: u8 = 12;

    /// Persistency discount rate (5-10%)
    pub const PERSISTENCY_DISCOUNT_MIN_BPS: u16 = 500;
    pub const PERSISTENCY_DISCOUNT_MAX_BPS: u16 = 1000;

    // =========================================================================
    // SECTION M: RUN-OFF RESERVE
    // Standard regulatory requirement for wind-down scenarios
    // =========================================================================

    /// Run-off reserve: 180 days IBNR + admin costs + legal costs
    /// Ensures claims can be paid even if Apollo ceases operations
    pub const RUNOFF_IBNR_DAYS: u16 = 180;
}

// =============================================================================
// BOOTSTRAP MODE CONFIGURATION
// =============================================================================
// Bootstrap mode enables conservative parameters for small-scale operation
// ($1.5M-$5M capital instead of $50M ICO assumption)

pub mod bootstrap {

    /// Maximum members in bootstrap phase (conservative for small capital)
    pub const BOOTSTRAP_MAX_MEMBERS: u32 = 200;

    /// Shock claim threshold during bootstrap (lower = more scrutiny)
    /// $25K vs $100K in scaled mode
    pub const BOOTSTRAP_SHOCK_THRESHOLD: u64 = 25_000_000_000;

    /// Auto-approve threshold during bootstrap (conservative)
    /// $500 vs $1,000 in scaled mode
    pub const BOOTSTRAP_AUTO_APPROVE: u64 = 500_000_000;

    /// Minimum reserve days before accepting new members
    pub const BOOTSTRAP_MIN_RESERVE_DAYS: u16 = 90;

    /// Enrollment cap in bootstrap (members per month)
    pub const BOOTSTRAP_ENROLLMENT_CAP: u32 = 25;

    /// Minimum capital to exit bootstrap mode
    pub const BOOTSTRAP_EXIT_CAPITAL: u64 = 2_000_000_000_000; // $2M

    /// Minimum members to exit bootstrap mode
    pub const BOOTSTRAP_EXIT_MEMBERS: u32 = 500;

    /// Calculate enrollment capacity based on reserve coverage
    /// Returns maximum new members per month
    pub fn calculate_enrollment_capacity(
        total_liquid_reserves: u64,
        expected_monthly_claims: u64,
    ) -> u32 {
        if expected_monthly_claims == 0 {
            return BOOTSTRAP_ENROLLMENT_CAP;
        }

        let months_coverage = total_liquid_reserves / expected_monthly_claims;

        match months_coverage {
            12.. => 100,  // 12+ months: up to 100/month
            6..=11 => 50, // 6-11 months: up to 50/month
            3..=5 => 10,  // 3-5 months: up to 10/month
            _ => 0,       // <3 months: frozen
        }
    }
}

pub mod protocol_constants {
    /// USDC decimals (6)
    pub const USDC_DECIMALS: u8 = 6;

    /// APH decimals (9)
    pub const APH_DECIMALS: u8 = 9;

    /// Seconds per day
    pub const SECONDS_PER_DAY: i64 = 86400;

    /// Seconds per month (30 days avg)
    pub const SECONDS_PER_MONTH: i64 = 2_592_000;

    /// Seconds per year (365 days)
    pub const SECONDS_PER_YEAR: i64 = 31_536_000;

    /// Maximum protocol fee (5%)
    pub const MAX_PROTOCOL_FEE_BPS: u16 = 500;

    /// Re-export BPS_DENOMINATOR for convenience
    pub const BPS_DENOMINATOR: u64 = super::actuarial::BPS_DENOMINATOR;
}

// =============================================================================
// EVENTS
// =============================================================================

#[event]
pub struct AphConfigInitialized {
    pub aph_mint: Pubkey,
    pub authority: Pubkey,
    pub transfer_fee_active: bool,
    pub timestamp: i64,
}

#[event]
pub struct TransferFeeUpdated {
    pub old_fee_bps: u16,
    pub new_fee_bps: u16,
    pub max_fee: u64,
    pub is_active: bool,
    pub timestamp: i64,
}

#[event]
pub struct VestingCreated {
    pub beneficiary: Pubkey,
    pub category: u8,
    pub total_amount: u64,
    pub cliff_end: i64,
    pub vesting_end: i64,
    pub timestamp: i64,
}

#[event]
pub struct VestingClaimed {
    pub beneficiary: Pubkey,
    pub amount: u64,
    pub total_claimed: u64,
    pub remaining: u64,
    pub timestamp: i64,
}

// =============================================================================
// ERRORS
// =============================================================================

#[error_code]
pub enum ApolloError {
    #[msg("Invalid APH token mint")]
    InvalidAphMint,

    #[msg("Transfer fee calculation overflow")]
    FeeCalculationOverflow,

    #[msg("Vesting cliff not reached")]
    CliffNotReached,

    #[msg("No tokens available to claim")]
    NothingToClaim,

    #[msg("Vesting has been revoked")]
    VestingRevoked,

    #[msg("Invalid allocation category")]
    InvalidCategory,

    #[msg("Unauthorized")]
    Unauthorized,

    #[msg("Token-2022 operation failed")]
    Token2022Error,
}

// =============================================================================
// PROGRAM ENTRYPOINT (minimal - mostly a library)
// =============================================================================

#[program]
pub mod apollo_core {
    use super::*;

    /// Initialize APH token configuration
    pub fn initialize_aph_config(
        ctx: Context<InitializeAphConfig>,
        transfer_fee_active: bool,
    ) -> Result<()> {
        let config = &mut ctx.accounts.aph_config;
        let clock = Clock::get()?;

        config.aph_mint = ctx.accounts.aph_mint.key();
        config.authority = ctx.accounts.authority.key();
        config.transfer_fee_active = transfer_fee_active;
        config.transfer_fee_bps = if transfer_fee_active {
            aph_token::TRANSFER_FEE_BPS
        } else {
            0
        };
        config.max_transfer_fee = aph_token::MAX_TRANSFER_FEE;
        config.fee_updated_at = clock.unix_timestamp;
        config.bump = ctx.bumps.aph_config;

        emit!(AphConfigInitialized {
            aph_mint: config.aph_mint,
            authority: config.authority,
            transfer_fee_active,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Update transfer fee status (for post-presale activation)
    pub fn update_transfer_fee_status(
        ctx: Context<UpdateTransferFeeStatus>,
        is_active: bool,
        fee_bps: u16,
    ) -> Result<()> {
        let config = &mut ctx.accounts.aph_config;
        let clock = Clock::get()?;

        let old_fee_bps = config.transfer_fee_bps;
        config.transfer_fee_active = is_active;
        config.transfer_fee_bps = fee_bps;
        config.fee_updated_at = clock.unix_timestamp;

        emit!(TransferFeeUpdated {
            old_fee_bps,
            new_fee_bps: fee_bps,
            max_fee: config.max_transfer_fee,
            is_active,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
}

// =============================================================================
// INSTRUCTION ACCOUNTS
// =============================================================================

#[derive(Accounts)]
pub struct InitializeAphConfig<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + AphTokenConfig::INIT_SPACE,
        seeds = [AphTokenConfig::SEED_PREFIX],
        bump
    )]
    pub aph_config: Account<'info, AphTokenConfig>,

    /// APH Token-2022 mint - validated against hardcoded address
    /// CHECK: Validated in instruction
    #[account(
        constraint = aph_mint.key() == aph_token::mint_pubkey() @ ApolloError::InvalidAphMint
    )]
    pub aph_mint: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateTransferFeeStatus<'info> {
    #[account(
        mut,
        seeds = [AphTokenConfig::SEED_PREFIX],
        bump = aph_config.bump,
        has_one = authority @ ApolloError::Unauthorized
    )]
    pub aph_config: Account<'info, AphTokenConfig>,

    pub authority: Signer<'info>,
}

// =============================================================================
// CPI HELPERS
// =============================================================================

/// Helper functions for other programs to interact with APH Token-2022
pub mod cpi_helpers {
    use super::*;

    use anchor_spl::token_interface;

    /// Transfer APH tokens using Token-2022 program
    /// Handles transfer fee extension if present
    pub fn transfer_aph<'info>(
        token_program: &AccountInfo<'info>,
        from: &AccountInfo<'info>,
        to: &AccountInfo<'info>,
        authority: &AccountInfo<'info>,
        mint: &AccountInfo<'info>,
        amount: u64,
        decimals: u8,
        signer_seeds: Option<&[&[&[u8]]]>,
    ) -> Result<()> {
        let cpi_accounts = token_interface::TransferChecked {
            from: from.clone(),
            to: to.clone(),
            authority: authority.clone(),
            mint: mint.clone(),
        };

        let cpi_ctx = if let Some(seeds) = signer_seeds {
            CpiContext::new_with_signer(token_program.clone(), cpi_accounts, seeds)
        } else {
            CpiContext::new(token_program.clone(), cpi_accounts)
        };

        token_interface::transfer_checked(cpi_ctx, amount, decimals)
    }

    /// Get seeds for APH config PDA
    pub fn get_aph_config_seeds() -> &'static [&'static [u8]] {
        &[AphTokenConfig::SEED_PREFIX]
    }
}
