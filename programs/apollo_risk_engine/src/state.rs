// programs/apollo_risk_engine/src/state.rs

use anchor_lang::prelude::*;

/// Risk engine configuration
/// PDA seeds: ["risk_config"]
#[account]
#[derive(InitSpace)]
pub struct RiskConfig {
    /// Authority (DAO)
    pub authority: Pubkey,

    /// Governance program for authorization
    pub governance_program: Pubkey,

    /// Reserves program for CAR data
    pub reserves_program: Pubkey,

    /// Base rate for adult coverage (USDC lamports per month)
    /// Default: $450 = 450_000_000 (6 decimals)
    pub base_rate_adult: u64,

    /// Child factor in basis points (4000 = 40% of adult)
    pub child_factor_bps: u16,

    /// Maximum children counted for pricing
    pub max_children: u8,

    /// Tobacco surcharge factor in basis points (12000 = 1.2x)
    pub tobacco_factor_bps: u16,

    /// Current ShockFactor in basis points (10000 = 1.0x)
    pub shock_factor_bps: u16,

    /// Maximum auto-adjustable ShockFactor (Yellow zone)
    pub max_auto_shock_factor_bps: u16,

    /// Maximum ShockFactor with Risk Committee approval (Orange zone)
    pub max_committee_shock_factor_bps: u16,

    /// Maximum ShockFactor with DAO emergency (Red zone)
    pub max_emergency_shock_factor_bps: u16,

    /// Minimum contribution (floor)
    pub min_contribution: u64,

    /// Is the risk engine active
    pub is_active: bool,

    /// Bump seed
    pub bump: u8,

    /// Reserved
    #[max_len(32)]
    pub reserved: Vec<u8>,
}

impl RiskConfig {
    pub const SEED_PREFIX: &'static [u8] = b"risk_config";

    // Default values per Actuarial Specification
    pub const DEFAULT_BASE_RATE: u64 = 450_000_000; // $450 USDC (30-44 band reference)
    pub const DEFAULT_CHILD_FACTOR_BPS: u16 = 4000; // 40% per child (capped at 3)
    pub const DEFAULT_MAX_CHILDREN: u8 = 3;
    pub const DEFAULT_TOBACCO_FACTOR_BPS: u16 = 15000; // 1.5x (50% surcharge, per spec)
    pub const DEFAULT_SHOCK_FACTOR_BPS: u16 = 10000; // 1.0x (normal operations)
    
    // ShockFactor limits per CAR zone (per Actuarial Spec)
    // These define maximum premium multipliers for each solvency zone
    pub const MAX_AUTO_SHOCK_BPS: u16 = 12000; // 1.2x (Yellow zone - auto adjustment)
    pub const MAX_COMMITTEE_SHOCK_BPS: u16 = 15000; // 1.5x (Orange zone - committee approval)
    pub const MAX_EMERGENCY_SHOCK_BPS: u16 = 20000; // 2.0x (Red zone - DAO emergency)
    // Note: Red zone 2.0x is an actuarial override from spec's 1.5x
    // Rationale: 50% increase insufficient for catastrophic scenarios
}

/// CMS-compliant age band rating table
/// PDA seeds: ["rating_table"]
#[account]
#[derive(InitSpace)]
pub struct RatingTable {
    /// Number of age bands defined
    pub band_count: u8,

    /// Age bands with their factors
    /// Each band: [min_age, max_age, factor_bps]
    /// Factor is in basis points (10000 = 1.0x)
    #[max_len(10)]
    pub age_bands: Vec<AgeBand>,

    /// Regional cost factors (by region code)
    #[max_len(20)]
    pub region_factors: Vec<RegionFactor>,

    /// Last updated timestamp
    pub last_updated: i64,

    /// Updater (must be Actuarial Committee)
    pub last_updater: Pubkey,

    /// Bump seed
    pub bump: u8,
}

impl RatingTable {
    pub const SEED_PREFIX: &'static [u8] = b"rating_table";

    /// Get age factor for a given age
    pub fn get_age_factor(&self, age: u8) -> u16 {
        for band in &self.age_bands {
            if age >= band.min_age && age <= band.max_age {
                return band.factor_bps;
            }
        }
        // Default to 1.0x if not found
        10000
    }

    /// Get region factor for a given region code
    pub fn get_region_factor(&self, region_code: u8) -> u16 {
        for region in &self.region_factors {
            if region.code == region_code {
                return region.factor_bps;
            }
        }
        // Default to 1.0x
        10000
    }
}

/// Age band definition (CMS-compliant 3:1 ratio max)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct AgeBand {
    pub min_age: u8,
    pub max_age: u8,
    /// Factor in basis points (10000 = 1.0x)
    /// CMS allows max 3:1 ratio (oldest to youngest)
    pub factor_bps: u16,
}

/// Regional cost factor
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct RegionFactor {
    /// Region code (0-255)
    pub code: u8,
    /// Factor in basis points
    pub factor_bps: u16,
}

/// Default CMS-compliant age bands
pub fn default_age_bands() -> Vec<AgeBand> {
    vec![
        AgeBand { min_age: 0, max_age: 20, factor_bps: 6350 },    // 0.635x
        AgeBand { min_age: 21, max_age: 24, factor_bps: 10000 },  // 1.0x (reference)
        AgeBand { min_age: 25, max_age: 29, factor_bps: 10040 },  // 1.004x
        AgeBand { min_age: 30, max_age: 34, factor_bps: 10130 },  // 1.013x
        AgeBand { min_age: 35, max_age: 39, factor_bps: 10460 },  // 1.046x
        AgeBand { min_age: 40, max_age: 44, factor_bps: 11350 },  // 1.135x
        AgeBand { min_age: 45, max_age: 49, factor_bps: 12780 },  // 1.278x
        AgeBand { min_age: 50, max_age: 54, factor_bps: 14870 },  // 1.487x
        AgeBand { min_age: 55, max_age: 59, factor_bps: 17060 },  // 1.706x
        AgeBand { min_age: 60, max_age: 64, factor_bps: 19050 },  // 1.905x (3:1 max)
    ]
}

/// Capital Adequacy Ratio (CAR) state
/// PDA seeds: ["car_state"]
#[account]
#[derive(InitSpace)]
pub struct CarState {
    /// Current CAR in basis points (12500 = 125%)
    pub current_car_bps: u16,

    /// Target CAR in basis points
    pub target_car_bps: u16,

    /// Minimum CAR threshold (triggers alerts)
    pub min_car_bps: u16,

    /// Total USDC reserves (from reserves program)
    pub total_usdc_reserves: u64,

    /// Eligible staked APH value in USDC (after haircut)
    pub eligible_aph_usdc: u64,

    /// Expected annual claims
    pub expected_annual_claims: u64,

    /// Current zone
    pub current_zone: Zone,

    /// Last CAR computation timestamp
    pub last_computed_at: i64,

    /// Bump seed
    pub bump: u8,
}

impl CarState {
    pub const SEED_PREFIX: &'static [u8] = b"car_state";
    pub const DEFAULT_TARGET_CAR: u16 = 12500; // 125%
    pub const DEFAULT_MIN_CAR: u16 = 10000; // 100%
    
    // Bootstrap constants
    /// Minimum members needed for credible CAR calculation
    pub const MIN_CREDIBLE_MEMBERS: u64 = 50;
    /// Average expected annual claims per member (for bootstrapping)
    /// Based on ~$450/mo contribution at 90% MLR = ~$4,860/year claims
    pub const AVG_ANNUAL_CLAIMS_PER_MEMBER: u64 = 4_860_000_000; // $4,860

    /// Compute CAR from inputs
    /// CAR = (Total Reserves + Eligible APH) / Expected Claims
    /// 
    /// BOOTSTRAP HANDLING: Returns u16::MAX when no members/claims
    /// This allows unlimited enrollment during initial launch phase
    pub fn compute_car(&self) -> u16 {
        // Bootstrap case: no expected claims = infinite capacity
        // This is correct because CAR is a ratio of reserves to liabilities
        // With no liabilities (members), reserves can support unlimited growth
        if self.expected_annual_claims == 0 {
            return u16::MAX; // Green zone - unlimited enrollment
        }

        let total_capital = self.total_usdc_reserves
            .saturating_add(self.eligible_aph_usdc);

        // CAR = (capital / claims) * 10000
        let car = total_capital
            .saturating_mul(10000)
            .checked_div(self.expected_annual_claims)
            .unwrap_or(u16::MAX as u64);

        // Cap at u16 max
        car.min(u16::MAX as u64) as u16
    }
    
    /// Compute CAR with explicit bootstrap handling
    /// Use this when member count is known for more accurate calculation
    /// 
    /// # Arguments
    /// * `member_count` - Current number of active members
    /// * `avg_annual_claim` - Optional override for average claims per member
    /// 
    /// # Returns
    /// * CAR in basis points (10000 = 100%)
    pub fn compute_car_with_members(
        &self,
        member_count: u64,
        avg_annual_claim: Option<u64>,
    ) -> u16 {
        // Bootstrap: no members = unlimited capacity
        if member_count == 0 {
            return u16::MAX;
        }
        
        // Use provided average or default
        let avg_claim = avg_annual_claim.unwrap_or(Self::AVG_ANNUAL_CLAIMS_PER_MEMBER);
        
        // Calculate expected annual claims from member count
        let expected_claims = member_count.saturating_mul(avg_claim);
        
        if expected_claims == 0 {
            return u16::MAX;
        }
        
        let total_capital = self.total_usdc_reserves
            .saturating_add(self.eligible_aph_usdc);
        
        let car = total_capital
            .saturating_mul(10000)
            .checked_div(expected_claims)
            .unwrap_or(u16::MAX as u64);
        
        car.min(u16::MAX as u64) as u16
    }
    
    /// Check if we have enough members for actuarially credible experience
    pub fn has_credible_experience(member_count: u64) -> bool {
        member_count >= Self::MIN_CREDIBLE_MEMBERS
    }

    /// Determine zone from CAR
    /// Per Actuarial Spec:
    /// - Green: CAR ≥ 150% (unlimited enrollment)
    /// - Yellow: 125% ≤ CAR < 150% (500/month max)
    /// - Orange: 100% ≤ CAR < 125% (100/month + enhanced underwriting)
    /// - Red: CAR < 100% (enrollment frozen)
    pub fn determine_zone(car_bps: u16) -> Zone {
        if car_bps >= 15000 {
            Zone::Green
        } else if car_bps >= 12500 {
            Zone::Yellow
        } else if car_bps >= 10000 {
            Zone::Orange
        } else {
            Zone::Red
        }
    }
}

/// Zone state - enrollment caps and restrictions
/// PDA seeds: ["zone_state"]
#[account]
#[derive(InitSpace)]
pub struct ZoneState {
    /// Current zone
    pub current_zone: Zone,

    /// Green zone CAR threshold (default: 150%)
    pub green_threshold_bps: u16,

    /// Yellow zone CAR threshold (default: 125%)
    pub yellow_threshold_bps: u16,

    /// Orange zone CAR threshold (default: 100%)
    pub orange_threshold_bps: u16,

    /// Green zone: unlimited enrollment
    pub green_enrollment_cap: u32,

    /// Yellow zone: max new members per month
    pub yellow_enrollment_cap: u32,

    /// Orange zone: max new members per month
    pub orange_enrollment_cap: u32,

    /// Current month enrollment count
    pub current_month_enrollments: u32,

    /// Month start timestamp
    pub month_start_timestamp: i64,

    /// Is enrollment frozen (Red zone)
    pub enrollment_frozen: bool,

    /// Last zone transition timestamp
    pub last_zone_change_at: i64,

    /// Bump seed
    pub bump: u8,
}

impl ZoneState {
    pub const SEED_PREFIX: &'static [u8] = b"zone_state";

    // Default thresholds
    pub const DEFAULT_GREEN_BPS: u16 = 15000; // 150%
    pub const DEFAULT_YELLOW_BPS: u16 = 12500; // 125%
    pub const DEFAULT_ORANGE_BPS: u16 = 10000; // 100%

    // Default caps
    pub const GREEN_CAP: u32 = u32::MAX; // Unlimited
    pub const YELLOW_CAP: u32 = 500; // Max 500/month
    pub const ORANGE_CAP: u32 = 100; // Max 100/month

    /// Check if enrollment is allowed
    pub fn can_enroll(&self) -> bool {
        if self.enrollment_frozen {
            return false;
        }

        let cap = match self.current_zone {
            Zone::Green => self.green_enrollment_cap,
            Zone::Yellow => self.yellow_enrollment_cap,
            Zone::Orange => self.orange_enrollment_cap,
            Zone::Red => 0,
        };

        self.current_month_enrollments < cap
    }

    /// Get current enrollment cap
    pub fn get_current_cap(&self) -> u32 {
        match self.current_zone {
            Zone::Green => self.green_enrollment_cap,
            Zone::Yellow => self.yellow_enrollment_cap,
            Zone::Orange => self.orange_enrollment_cap,
            Zone::Red => 0,
        }
    }
}

/// Zone classification based on CAR
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum Zone {
    /// CAR > 150%: Unlimited enrollment, normal operations
    Green,
    /// CAR 125-150%: Limited enrollment (500/month), ShockFactor up to 1.25x auto
    Yellow,
    /// CAR 100-125%: Restricted enrollment (100/month), ShockFactor up to 1.5x with committee
    Orange,
    /// CAR < 100%: Enrollment frozen, ShockFactor up to 2.0x with DAO emergency
    Red,
}

impl Default for Zone {
    fn default() -> Self {
        Zone::Green
    }
}

/// Member risk tier for internal classification
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum MemberRiskTier {
    /// Low risk - healthy, young, no pre-existing conditions
    Low,
    /// Standard risk - average health profile
    Standard,
    /// Elevated risk - some risk factors present
    Elevated,
    /// High risk - multiple risk factors or chronic conditions
    High,
}

impl Default for MemberRiskTier {
    fn default() -> Self {
        MemberRiskTier::Standard
    }
}

/// Contribution quote result
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ContributionQuote {
    /// Base contribution before loads
    pub base_amount: u64,
    /// Age factor applied (bps)
    pub age_factor_bps: u16,
    /// Region factor applied (bps)
    pub region_factor_bps: u16,
    /// Tobacco factor applied (bps)
    pub tobacco_factor_bps: u16,
    /// ShockFactor applied (bps)
    pub shock_factor_bps: u16,
    /// Final monthly contribution
    pub final_contribution: u64,
    /// Computed at timestamp
    pub quoted_at: i64,
}
