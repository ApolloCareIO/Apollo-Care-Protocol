// programs/apollo_membership/src/instructions/coverage.rs

use anchor_lang::prelude::*;
use crate::state::{GlobalConfig, MemberAccount, MemberStatus, ContributionLedger};
use crate::errors::MembershipError;
use crate::events::{CoverageActivated, MemberStatusChanged, MemberSuspended, MemberTerminated};

/// Activate coverage after waiting period
#[derive(Accounts)]
pub struct ActivateCoverageIfEligible<'info> {
    #[account(
        mut,
        seeds = [GlobalConfig::SEED_PREFIX],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
        mut,
        seeds = [MemberAccount::SEED_PREFIX, member_account.member.as_ref()],
        bump = member_account.bump,
        constraint = member_account.status == MemberStatus::PendingActivation @ MembershipError::CannotActivate
    )]
    pub member_account: Account<'info, MemberAccount>,

    #[account(
        seeds = [ContributionLedger::SEED_PREFIX, member_account.member.as_ref()],
        bump = contribution_ledger.bump,
    )]
    pub contribution_ledger: Account<'info, ContributionLedger>,
}

pub fn activate_coverage_if_eligible(ctx: Context<ActivateCoverageIfEligible>) -> Result<()> {
    let clock = Clock::get()?;
    let config = &mut ctx.accounts.global_config;
    let member = &mut ctx.accounts.member_account;
    let ledger = &ctx.accounts.contribution_ledger;

    // Check waiting period has passed
    let waiting_days = config.default_waiting_period_days as i64;
    let waiting_period_end = member.enrolled_at + (waiting_days * 24 * 60 * 60);

    require!(
        clock.unix_timestamp >= waiting_period_end,
        MembershipError::WaitingPeriodNotComplete
    );

    // Check first payment was made
    require!(
        ledger.total_deposits > 0,
        MembershipError::InsufficientContribution
    );

    // Activate coverage
    let old_status = member.status;
    member.status = MemberStatus::Active;
    member.coverage_activated_at = clock.unix_timestamp;

    config.active_members += 1;

    emit!(CoverageActivated {
        member: member.member,
        activated_at: clock.unix_timestamp,
        timestamp: clock.unix_timestamp,
    });

    emit!(MemberStatusChanged {
        member: member.member,
        old_status,
        new_status: MemberStatus::Active,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Suspend coverage for non-payment
#[derive(Accounts)]
pub struct SuspendCoverage<'info> {
    #[account(
        mut,
        seeds = [GlobalConfig::SEED_PREFIX],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
        mut,
        seeds = [MemberAccount::SEED_PREFIX, member_account.member.as_ref()],
        bump = member_account.bump,
        constraint = member_account.status == MemberStatus::Active @ MembershipError::CannotSuspend
    )]
    pub member_account: Account<'info, MemberAccount>,

    #[account(
        seeds = [ContributionLedger::SEED_PREFIX, member_account.member.as_ref()],
        bump = contribution_ledger.bump,
    )]
    pub contribution_ledger: Account<'info, ContributionLedger>,

    /// Can be called by system or authority
    pub suspender: Signer<'info>,
}

pub fn suspend_coverage(ctx: Context<SuspendCoverage>, reason: String) -> Result<()> {
    let clock = Clock::get()?;
    let config = &mut ctx.accounts.global_config;
    let member = &mut ctx.accounts.member_account;
    let ledger = &ctx.accounts.contribution_ledger;

    // Verify payment is overdue (more than 30 days past due)
    let grace_period = 30 * 24 * 60 * 60; // 30 days
    require!(
        clock.unix_timestamp > ledger.next_payment_due + grace_period,
        MembershipError::PaymentOverdue
    );

    let old_status = member.status;
    member.status = MemberStatus::Suspended;
    member.consecutive_months = 0; // Reset streak

    config.active_members = config.active_members.saturating_sub(1);

    emit!(MemberSuspended {
        member: member.member,
        reason,
        timestamp: clock.unix_timestamp,
    });

    emit!(MemberStatusChanged {
        member: member.member,
        old_status,
        new_status: MemberStatus::Suspended,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Reinstate suspended coverage
#[derive(Accounts)]
pub struct ReinstateCoverage<'info> {
    #[account(
        mut,
        seeds = [GlobalConfig::SEED_PREFIX],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
        mut,
        seeds = [MemberAccount::SEED_PREFIX, member.key().as_ref()],
        bump = member_account.bump,
        constraint = member_account.status == MemberStatus::Suspended @ MembershipError::CannotActivate,
        constraint = member_account.member == member.key() @ MembershipError::Unauthorized
    )]
    pub member_account: Account<'info, MemberAccount>,

    #[account(
        seeds = [ContributionLedger::SEED_PREFIX, member.key().as_ref()],
        bump = contribution_ledger.bump,
    )]
    pub contribution_ledger: Account<'info, ContributionLedger>,

    pub member: Signer<'info>,
}

pub fn reinstate_coverage(ctx: Context<ReinstateCoverage>) -> Result<()> {
    let clock = Clock::get()?;
    let config = &mut ctx.accounts.global_config;
    let member = &mut ctx.accounts.member_account;
    let ledger = &ctx.accounts.contribution_ledger;

    // Must have made a payment to reinstate
    // Payment should bring them current or ahead
    require!(
        ledger.balance >= ledger.amount_due ||
        clock.unix_timestamp <= ledger.next_payment_due,
        MembershipError::InsufficientContribution
    );

    let old_status = member.status;
    member.status = MemberStatus::Active;

    config.active_members += 1;

    emit!(MemberStatusChanged {
        member: member.member,
        old_status,
        new_status: MemberStatus::Active,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Terminate coverage
#[derive(Accounts)]
pub struct TerminateCoverage<'info> {
    #[account(
        mut,
        seeds = [GlobalConfig::SEED_PREFIX],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
        mut,
        seeds = [MemberAccount::SEED_PREFIX, member_account.member.as_ref()],
        bump = member_account.bump,
    )]
    pub member_account: Account<'info, MemberAccount>,

    /// Must be authority or member
    pub terminator: Signer<'info>,
}

pub fn terminate_coverage(ctx: Context<TerminateCoverage>, reason: String) -> Result<()> {
    let clock = Clock::get()?;
    let config = &mut ctx.accounts.global_config;
    let member = &mut ctx.accounts.member_account;

    // Cannot terminate already terminated
    require!(
        member.status != MemberStatus::Terminated,
        MembershipError::CannotTerminate
    );

    let old_status = member.status;

    // Decrement active count if was active
    if old_status == MemberStatus::Active {
        config.active_members = config.active_members.saturating_sub(1);
    }

    member.status = MemberStatus::Terminated;

    emit!(MemberTerminated {
        member: member.member,
        reason,
        timestamp: clock.unix_timestamp,
    });

    emit!(MemberStatusChanged {
        member: member.member,
        old_status,
        new_status: MemberStatus::Terminated,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Get member coverage status
#[derive(Accounts)]
pub struct GetMemberStatus<'info> {
    #[account(
        seeds = [MemberAccount::SEED_PREFIX, member_account.member.as_ref()],
        bump = member_account.bump,
    )]
    pub member_account: Account<'info, MemberAccount>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct MemberCoverageStatus {
    pub status: MemberStatus,
    pub enrolled_at: i64,
    pub coverage_activated_at: i64,
    pub consecutive_months: u16,
    pub monthly_contribution: u64,
    pub persistency_discount_bps: u16,
    pub has_qualifying_event: bool,
}

pub fn get_member_status(ctx: Context<GetMemberStatus>) -> Result<MemberCoverageStatus> {
    let member = &ctx.accounts.member_account;

    Ok(MemberCoverageStatus {
        status: member.status,
        enrolled_at: member.enrolled_at,
        coverage_activated_at: member.coverage_activated_at,
        consecutive_months: member.consecutive_months,
        monthly_contribution: member.monthly_contribution,
        persistency_discount_bps: member.persistency_discount_bps,
        has_qualifying_event: member.has_qualifying_event,
    })
}
