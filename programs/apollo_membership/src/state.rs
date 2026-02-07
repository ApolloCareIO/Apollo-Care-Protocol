// programs/apollo_membership/src/state.rs

use anchor_lang::prelude::*;

/// Global membership configuration
/// PDA seeds: ["global_config"]
#[account]
#[derive(InitSpace)]
pub struct GlobalConfig {
    /// Authority (DAO)
    pub authority: Pubkey,

    /// Governance program
    pub governance_program: Pubkey,

    /// Risk engine program (for contribution quotes)
    pub risk_engine_program: Pubkey,

    /// Reserves program (for contribution routing)
    pub reserves_program: Pubkey,

    /// USDC mint
    pub usdc_mint: Pubkey,

    /// Total members enrolled
    pub total_members: u64,

    /// Total active members
    pub active_members: u64,

    /// Total contributions received (all-time)
    pub total_contributions: u64,

    /// Default waiting period (days) before coverage activates
    pub default_waiting_period_days: u16,

    /// Pre-existing condition waiting period (days)
    pub preexisting_waiting_days: u16,

    /// Persistency discount start (months of continuous coverage)
    pub persistency_discount_start_months: u8,

    /// Persistency discount per year (basis points)
    pub persistency_discount_bps: u16,

    /// Maximum persistency discount (basis points)
    pub max_persistency_discount_bps: u16,

    /// Is enrollment open
    pub enrollment_open: bool,

    /// Bump seed
    pub bump: u8,
}

impl GlobalConfig {
    pub const SEED_PREFIX: &'static [u8] = b"global_config";

    pub const DEFAULT_WAITING_PERIOD: u16 = 30; // 30 days
    pub const DEFAULT_PREEXISTING_WAIT: u16 = 180; // 6 months
    pub const DEFAULT_PERSISTENCY_START: u8 = 12; // After 1 year
    pub const DEFAULT_PERSISTENCY_BPS: u16 = 500; // 5% per year
    pub const MAX_PERSISTENCY_BPS: u16 = 1000; // Max 10%
}

/// Individual member account
/// PDA seeds: ["member", member_pubkey]
#[account]
#[derive(InitSpace)]
pub struct MemberAccount {
    /// Member's wallet address
    pub member: Pubkey,

    /// Member ID (sequential)
    pub member_id: u64,

    /// Primary member age
    pub age: u8,

    /// Region code for pricing
    pub region_code: u8,

    /// Is tobacco user
    pub is_tobacco_user: bool,

    /// Number of dependents (children)
    pub num_children: u8,

    /// Number of additional adults
    pub num_additional_adults: u8,

    /// Enrollment timestamp
    pub enrolled_at: i64,

    /// Coverage activation timestamp (0 if not yet active)
    pub coverage_activated_at: i64,

    /// Coverage status
    pub status: MemberStatus,

    /// Current monthly contribution amount
    pub monthly_contribution: u64,

    /// Total contributions paid
    pub total_contributions_paid: u64,

    /// Last contribution timestamp
    pub last_contribution_at: i64,

    /// Consecutive months of coverage
    pub consecutive_months: u16,

    /// Has qualifying life event
    pub has_qualifying_event: bool,

    /// Qualifying event timestamp
    pub qualifying_event_at: i64,

    /// Applied persistency discount (basis points)
    pub persistency_discount_bps: u16,

    /// Benefit schedule key
    #[max_len(32)]
    pub benefit_schedule: String,

    /// Bump seed
    pub bump: u8,
}

impl MemberAccount {
    pub const SEED_PREFIX: &'static [u8] = b"member";
}

/// Member status
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum MemberStatus {
    /// Just enrolled, waiting period not complete
    PendingActivation,
    /// Coverage is active
    Active,
    /// Coverage suspended (missed payment)
    Suspended,
    /// Coverage terminated
    Terminated,
    /// Coverage lapsed (extended non-payment)
    Lapsed,
}

impl Default for MemberStatus {
    fn default() -> Self {
        MemberStatus::PendingActivation
    }
}

/// Enrollment window configuration
/// PDA seeds: ["enrollment_window", window_id]
#[account]
#[derive(InitSpace)]
pub struct EnrollmentWindow {
    /// Window ID
    pub window_id: u64,

    /// Window start timestamp
    pub start_time: i64,

    /// Window end timestamp
    pub end_time: i64,

    /// Maximum enrollments in this window
    pub max_enrollments: u32,

    /// Current enrollment count
    pub enrollment_count: u32,

    /// Is this window active
    pub is_active: bool,

    /// Is this a special enrollment period
    pub is_special_enrollment: bool,

    /// Description
    #[max_len(64)]
    pub description: String,

    /// Bump seed
    pub bump: u8,
}

impl EnrollmentWindow {
    pub const SEED_PREFIX: &'static [u8] = b"enrollment_window";

    pub fn is_open(&self, current_time: i64) -> bool {
        self.is_active
            && current_time >= self.start_time
            && current_time <= self.end_time
            && self.enrollment_count < self.max_enrollments
    }
}

/// Contribution ledger for tracking payments
/// PDA seeds: ["contribution_ledger", member]
#[account]
#[derive(InitSpace)]
pub struct ContributionLedger {
    /// Member this ledger belongs to
    pub member: Pubkey,

    /// Current balance (credit from overpayment)
    pub balance: u64,

    /// Total deposits
    pub total_deposits: u64,

    /// Total applied to coverage
    pub total_applied: u64,

    /// Last deposit timestamp
    pub last_deposit_at: i64,

    /// Next payment due timestamp
    pub next_payment_due: i64,

    /// Amount due for next period
    pub amount_due: u64,

    /// Number of on-time payments
    pub on_time_payments: u32,

    /// Number of late payments
    pub late_payments: u32,

    /// Bump seed
    pub bump: u8,
}

impl ContributionLedger {
    pub const SEED_PREFIX: &'static [u8] = b"contribution_ledger";
}

/// Qualifying life events for special enrollment
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum QualifyingEvent {
    /// Loss of other coverage
    LossOfCoverage,
    /// Marriage
    Marriage,
    /// Birth/Adoption of child
    BirthAdoption,
    /// Divorce
    Divorce,
    /// Move to new coverage area
    Relocation,
    /// Change in income (for subsidies)
    IncomeChange,
    /// Other (requires approval)
    Other,
}

// ==================== UNIT TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== GLOBAL CONFIG TESTS ====================

    #[test]
    fn test_global_config_defaults() {
        assert_eq!(GlobalConfig::DEFAULT_WAITING_PERIOD, 30);
        assert_eq!(GlobalConfig::DEFAULT_PREEXISTING_WAIT, 180);
        assert_eq!(GlobalConfig::DEFAULT_PERSISTENCY_START, 12);
        assert_eq!(GlobalConfig::DEFAULT_PERSISTENCY_BPS, 500); // 5%
        assert_eq!(GlobalConfig::MAX_PERSISTENCY_BPS, 1000); // 10%
    }

    #[test]
    fn test_global_config_seed_prefix() {
        assert_eq!(GlobalConfig::SEED_PREFIX, b"global_config");
    }

    // ==================== MEMBER ACCOUNT TESTS ====================

    #[test]
    fn test_member_account_seed_prefix() {
        assert_eq!(MemberAccount::SEED_PREFIX, b"member");
    }

    // ==================== MEMBER STATUS TESTS ====================

    #[test]
    fn test_member_status_default() {
        let status = MemberStatus::default();
        assert_eq!(status, MemberStatus::PendingActivation);
    }

    #[test]
    fn test_member_status_equality() {
        assert_eq!(MemberStatus::Active, MemberStatus::Active);
        assert_ne!(MemberStatus::Active, MemberStatus::Suspended);
        assert_ne!(MemberStatus::PendingActivation, MemberStatus::Terminated);
    }

    #[test]
    fn test_all_member_statuses() {
        // Ensure all variants can be created
        let _ = MemberStatus::PendingActivation;
        let _ = MemberStatus::Active;
        let _ = MemberStatus::Suspended;
        let _ = MemberStatus::Terminated;
        let _ = MemberStatus::Lapsed;
    }

    // ==================== ENROLLMENT WINDOW TESTS ====================

    fn create_test_enrollment_window(
        start: i64,
        end: i64,
        active: bool,
        count: u32,
        max: u32,
    ) -> EnrollmentWindow {
        EnrollmentWindow {
            window_id: 1,
            start_time: start,
            end_time: end,
            max_enrollments: max,
            enrollment_count: count,
            is_active: active,
            is_special_enrollment: false,
            description: String::from("Test Window"),
            bump: 255,
        }
    }

    #[test]
    fn test_enrollment_window_is_open_true() {
        let window = create_test_enrollment_window(100, 200, true, 50, 1000);
        assert!(window.is_open(150));
    }

    #[test]
    fn test_enrollment_window_is_open_before_start() {
        let window = create_test_enrollment_window(100, 200, true, 50, 1000);
        assert!(!window.is_open(50));
    }

    #[test]
    fn test_enrollment_window_is_open_after_end() {
        let window = create_test_enrollment_window(100, 200, true, 50, 1000);
        assert!(!window.is_open(250));
    }

    #[test]
    fn test_enrollment_window_is_open_inactive() {
        let window = create_test_enrollment_window(100, 200, false, 50, 1000);
        assert!(!window.is_open(150));
    }

    #[test]
    fn test_enrollment_window_is_open_at_cap() {
        let window = create_test_enrollment_window(100, 200, true, 1000, 1000);
        assert!(!window.is_open(150));
    }

    #[test]
    fn test_enrollment_window_is_open_at_boundaries() {
        let window = create_test_enrollment_window(100, 200, true, 50, 1000);
        assert!(window.is_open(100)); // At start
        assert!(window.is_open(200)); // At end
    }

    #[test]
    fn test_enrollment_window_seed_prefix() {
        assert_eq!(EnrollmentWindow::SEED_PREFIX, b"enrollment_window");
    }

    // ==================== CONTRIBUTION LEDGER TESTS ====================

    #[test]
    fn test_contribution_ledger_seed_prefix() {
        assert_eq!(ContributionLedger::SEED_PREFIX, b"contribution_ledger");
    }

    // ==================== QUALIFYING EVENT TESTS ====================

    #[test]
    fn test_qualifying_event_equality() {
        assert_eq!(QualifyingEvent::Marriage, QualifyingEvent::Marriage);
        assert_ne!(QualifyingEvent::Marriage, QualifyingEvent::Divorce);
    }

    #[test]
    fn test_all_qualifying_events() {
        let _ = QualifyingEvent::LossOfCoverage;
        let _ = QualifyingEvent::Marriage;
        let _ = QualifyingEvent::BirthAdoption;
        let _ = QualifyingEvent::Divorce;
        let _ = QualifyingEvent::Relocation;
        let _ = QualifyingEvent::IncomeChange;
        let _ = QualifyingEvent::Other;
    }

    // ==================== PERSISTENCY DISCOUNT TESTS ====================

    #[test]
    fn test_persistency_discount_calculation() {
        let discount_bps = GlobalConfig::DEFAULT_PERSISTENCY_BPS;
        let max_discount_bps = GlobalConfig::MAX_PERSISTENCY_BPS;

        // Year 1: 5%
        assert_eq!(discount_bps, 500);

        // Year 2: 10% (capped)
        let year2 = std::cmp::min(discount_bps * 2, max_discount_bps);
        assert_eq!(year2, 1000);

        // Year 3+: still 10%
        let year3 = std::cmp::min(discount_bps * 3, max_discount_bps);
        assert_eq!(year3, 1000);
    }
}
