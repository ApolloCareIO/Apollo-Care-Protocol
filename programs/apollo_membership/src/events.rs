// programs/apollo_membership/src/events.rs

use anchor_lang::prelude::*;
use crate::state::{MemberStatus, QualifyingEvent};

/// Emitted when global config is initialized
#[event]
pub struct GlobalConfigInitialized {
    pub authority: Pubkey,
    pub usdc_mint: Pubkey,
    pub timestamp: i64,
}

/// Emitted when an enrollment window is opened
#[event]
pub struct EnrollmentWindowOpened {
    pub window_id: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub max_enrollments: u32,
    pub is_special: bool,
    pub timestamp: i64,
}

/// Emitted when an enrollment window is closed
#[event]
pub struct EnrollmentWindowClosed {
    pub window_id: u64,
    pub final_enrollment_count: u32,
    pub timestamp: i64,
}

/// Emitted when a member enrolls
#[event]
pub struct MemberEnrolled {
    pub member: Pubkey,
    pub member_id: u64,
    pub age: u8,
    pub region_code: u8,
    pub monthly_contribution: u64,
    pub waiting_period_ends: i64,
    pub timestamp: i64,
}

/// Emitted when coverage is activated
#[event]
pub struct CoverageActivated {
    pub member: Pubkey,
    pub activated_at: i64,
    pub timestamp: i64,
}

/// Emitted when a contribution is deposited
#[event]
pub struct ContributionDeposited {
    pub member: Pubkey,
    pub amount: u64,
    pub total_contributions: u64,
    pub next_payment_due: i64,
    pub timestamp: i64,
}

/// Emitted when member status changes
#[event]
pub struct MemberStatusChanged {
    pub member: Pubkey,
    pub old_status: MemberStatus,
    pub new_status: MemberStatus,
    pub timestamp: i64,
}

/// Emitted when a qualifying event is set
#[event]
pub struct QualifyingEventSet {
    pub member: Pubkey,
    pub event_type: QualifyingEvent,
    pub expires_at: i64,
    pub timestamp: i64,
}

/// Emitted when persistency discount is applied
#[event]
pub struct PersistencyDiscountApplied {
    pub member: Pubkey,
    pub consecutive_months: u16,
    pub discount_bps: u16,
    pub new_contribution: u64,
    pub timestamp: i64,
}

/// Emitted when member is suspended
#[event]
pub struct MemberSuspended {
    pub member: Pubkey,
    pub reason: String,
    pub timestamp: i64,
}

/// Emitted when member is terminated
#[event]
pub struct MemberTerminated {
    pub member: Pubkey,
    pub reason: String,
    pub timestamp: i64,
}

/// Emitted when member info is updated
#[event]
pub struct MemberInfoUpdated {
    pub member: Pubkey,
    pub field: String,
    pub timestamp: i64,
}
