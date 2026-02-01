// programs/apollo_membership/src/lib.rs
//
// Apollo Membership Program
// =========================
// Manages member lifecycle:
// - Enrollment windows (open/special enrollment)
// - Member registration and profiles
// - Contribution collection and routing
// - Coverage activation and management
// - Persistency discounts for loyal members

use anchor_lang::prelude::*;

pub mod state;
pub mod errors;
pub mod events;
pub mod instructions;

use instructions::*;
use state::QualifyingEvent;

declare_id!("CHJ4Bdc9wqKy6pjSiC3URjs53iDQpn58MPeAgLQVqRW1");

#[program]
pub mod apollo_membership {
    use super::*;

    // ==================== INITIALIZATION ====================

    /// Initialize global membership configuration
    pub fn initialize_global_config(
        ctx: Context<InitializeGlobalConfig>,
        params: InitializeGlobalConfigParams,
    ) -> Result<()> {
        instructions::initialize::handler(ctx, params)
    }

    // ==================== ENROLLMENT WINDOWS ====================

    /// Open an enrollment window
    pub fn open_enrollment_window(
        ctx: Context<OpenEnrollmentWindow>,
        params: OpenEnrollmentWindowParams,
    ) -> Result<()> {
        instructions::enrollment::open_enrollment_window(ctx, params)
    }

    /// Close an enrollment window
    pub fn close_enrollment_window(ctx: Context<CloseEnrollmentWindow>) -> Result<()> {
        instructions::enrollment::close_enrollment_window(ctx)
    }

    /// Enroll a new member
    pub fn enroll_member(ctx: Context<EnrollMember>, params: EnrollMemberParams) -> Result<()> {
        instructions::enrollment::enroll_member(ctx, params)
    }

    /// Set a qualifying life event
    pub fn set_member_qualifying_event(
        ctx: Context<SetMemberQualifyingEvent>,
        event_type: QualifyingEvent,
    ) -> Result<()> {
        instructions::enrollment::set_member_qualifying_event(ctx, event_type)
    }

    // ==================== CONTRIBUTIONS ====================

    /// Deposit a contribution
    pub fn deposit_contribution(ctx: Context<DepositContribution>, amount: u64) -> Result<()> {
        instructions::contributions::deposit_contribution(ctx, amount)
    }

    /// Apply persistency discount
    pub fn apply_persistency_discount(ctx: Context<ApplyPersistencyDiscount>) -> Result<()> {
        instructions::contributions::apply_persistency_discount(ctx)
    }

    /// Check payment status
    pub fn check_payment_status(ctx: Context<CheckPaymentStatus>) -> Result<PaymentStatus> {
        instructions::contributions::check_payment_status(ctx)
    }

    // ==================== COVERAGE MANAGEMENT ====================

    /// Activate coverage after waiting period
    pub fn activate_coverage_if_eligible(ctx: Context<ActivateCoverageIfEligible>) -> Result<()> {
        instructions::coverage::activate_coverage_if_eligible(ctx)
    }

    /// Suspend coverage for non-payment
    pub fn suspend_coverage(ctx: Context<SuspendCoverage>, reason: String) -> Result<()> {
        instructions::coverage::suspend_coverage(ctx, reason)
    }

    /// Reinstate suspended coverage
    pub fn reinstate_coverage(ctx: Context<ReinstateCoverage>) -> Result<()> {
        instructions::coverage::reinstate_coverage(ctx)
    }

    /// Terminate coverage
    pub fn terminate_coverage(ctx: Context<TerminateCoverage>, reason: String) -> Result<()> {
        instructions::coverage::terminate_coverage(ctx, reason)
    }

    /// Get member coverage status
    pub fn get_member_status(ctx: Context<GetMemberStatus>) -> Result<MemberCoverageStatus> {
        instructions::coverage::get_member_status(ctx)
    }
}
