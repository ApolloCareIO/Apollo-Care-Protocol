// programs/apollo_membership/src/instructions/enrollment.rs

use anchor_lang::prelude::*;
use crate::state::{GlobalConfig, MemberAccount, MemberStatus, EnrollmentWindow, ContributionLedger, QualifyingEvent};
use crate::errors::MembershipError;
use crate::events::{EnrollmentWindowOpened, EnrollmentWindowClosed, MemberEnrolled, QualifyingEventSet};

/// Open an enrollment window
#[derive(Accounts)]
#[instruction(window_id: u64)]
pub struct OpenEnrollmentWindow<'info> {
    #[account(
        mut,
        seeds = [GlobalConfig::SEED_PREFIX],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
        init,
        payer = authority,
        space = 8 + EnrollmentWindow::INIT_SPACE,
        seeds = [EnrollmentWindow::SEED_PREFIX, &window_id.to_le_bytes()],
        bump
    )]
    pub enrollment_window: Account<'info, EnrollmentWindow>,

    #[account(
        mut,
        constraint = authority.key() == global_config.authority @ MembershipError::Unauthorized
    )]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct OpenEnrollmentWindowParams {
    pub window_id: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub max_enrollments: u32,
    pub is_special_enrollment: bool,
    pub description: String,
}

pub fn open_enrollment_window(
    ctx: Context<OpenEnrollmentWindow>,
    params: OpenEnrollmentWindowParams,
) -> Result<()> {
    let clock = Clock::get()?;

    require!(params.end_time > params.start_time, MembershipError::InvalidWindowConfig);
    require!(params.max_enrollments > 0, MembershipError::InvalidWindowConfig);

    let window = &mut ctx.accounts.enrollment_window;
    window.window_id = params.window_id;
    window.start_time = params.start_time;
    window.end_time = params.end_time;
    window.max_enrollments = params.max_enrollments;
    window.enrollment_count = 0;
    window.is_active = true;
    window.is_special_enrollment = params.is_special_enrollment;
    window.description = params.description;
    window.bump = ctx.bumps.enrollment_window;

    ctx.accounts.global_config.enrollment_open = true;

    emit!(EnrollmentWindowOpened {
        window_id: params.window_id,
        start_time: params.start_time,
        end_time: params.end_time,
        max_enrollments: params.max_enrollments,
        is_special: params.is_special_enrollment,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Close an enrollment window
#[derive(Accounts)]
pub struct CloseEnrollmentWindow<'info> {
    #[account(
        mut,
        seeds = [GlobalConfig::SEED_PREFIX],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
        mut,
        seeds = [EnrollmentWindow::SEED_PREFIX, &enrollment_window.window_id.to_le_bytes()],
        bump = enrollment_window.bump,
    )]
    pub enrollment_window: Account<'info, EnrollmentWindow>,

    #[account(
        constraint = authority.key() == global_config.authority @ MembershipError::Unauthorized
    )]
    pub authority: Signer<'info>,
}

pub fn close_enrollment_window(ctx: Context<CloseEnrollmentWindow>) -> Result<()> {
    let clock = Clock::get()?;
    let window = &mut ctx.accounts.enrollment_window;

    window.is_active = false;

    emit!(EnrollmentWindowClosed {
        window_id: window.window_id,
        final_enrollment_count: window.enrollment_count,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Enroll a new member
#[derive(Accounts)]
pub struct EnrollMember<'info> {
    #[account(
        mut,
        seeds = [GlobalConfig::SEED_PREFIX],
        bump = global_config.bump,
        constraint = global_config.enrollment_open @ MembershipError::EnrollmentClosed
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
        mut,
        seeds = [EnrollmentWindow::SEED_PREFIX, &enrollment_window.window_id.to_le_bytes()],
        bump = enrollment_window.bump,
    )]
    pub enrollment_window: Account<'info, EnrollmentWindow>,

    #[account(
        init,
        payer = member,
        space = 8 + MemberAccount::INIT_SPACE,
        seeds = [MemberAccount::SEED_PREFIX, member.key().as_ref()],
        bump
    )]
    pub member_account: Account<'info, MemberAccount>,

    #[account(
        init,
        payer = member,
        space = 8 + ContributionLedger::INIT_SPACE,
        seeds = [ContributionLedger::SEED_PREFIX, member.key().as_ref()],
        bump
    )]
    pub contribution_ledger: Account<'info, ContributionLedger>,

    #[account(mut)]
    pub member: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct EnrollMemberParams {
    pub age: u8,
    pub region_code: u8,
    pub is_tobacco_user: bool,
    pub num_children: u8,
    pub num_additional_adults: u8,
    pub benefit_schedule: String,
    pub quoted_contribution: u64,
}

pub fn enroll_member(ctx: Context<EnrollMember>, params: EnrollMemberParams) -> Result<()> {
    let clock = Clock::get()?;
    let config = &mut ctx.accounts.global_config;
    let window = &mut ctx.accounts.enrollment_window;

    // Validate window is open
    require!(window.is_open(clock.unix_timestamp), MembershipError::WindowNotActive);
    require!(params.age > 0 && params.age <= 64, MembershipError::InvalidAge);

    // Increment counts
    window.enrollment_count += 1;
    config.total_members += 1;
    let member_id = config.total_members;

    // Calculate waiting period end
    let waiting_days = config.default_waiting_period_days as i64;
    let waiting_period_ends = clock.unix_timestamp + (waiting_days * 24 * 60 * 60);

    // Initialize member account
    let member_account = &mut ctx.accounts.member_account;
    member_account.member = ctx.accounts.member.key();
    member_account.member_id = member_id;
    member_account.age = params.age;
    member_account.region_code = params.region_code;
    member_account.is_tobacco_user = params.is_tobacco_user;
    member_account.num_children = params.num_children;
    member_account.num_additional_adults = params.num_additional_adults;
    member_account.enrolled_at = clock.unix_timestamp;
    member_account.coverage_activated_at = 0;
    member_account.status = MemberStatus::PendingActivation;
    member_account.monthly_contribution = params.quoted_contribution;
    member_account.total_contributions_paid = 0;
    member_account.last_contribution_at = 0;
    member_account.consecutive_months = 0;
    member_account.has_qualifying_event = false;
    member_account.qualifying_event_at = 0;
    member_account.persistency_discount_bps = 0;
    member_account.benefit_schedule = params.benefit_schedule;
    member_account.bump = ctx.bumps.member_account;

    // Initialize contribution ledger
    let ledger = &mut ctx.accounts.contribution_ledger;
    ledger.member = ctx.accounts.member.key();
    ledger.balance = 0;
    ledger.total_deposits = 0;
    ledger.total_applied = 0;
    ledger.last_deposit_at = 0;
    ledger.next_payment_due = clock.unix_timestamp; // First payment due now
    ledger.amount_due = params.quoted_contribution;
    ledger.on_time_payments = 0;
    ledger.late_payments = 0;
    ledger.bump = ctx.bumps.contribution_ledger;

    emit!(MemberEnrolled {
        member: ctx.accounts.member.key(),
        member_id,
        age: params.age,
        region_code: params.region_code,
        monthly_contribution: params.quoted_contribution,
        waiting_period_ends,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Set a qualifying life event for special enrollment
#[derive(Accounts)]
pub struct SetMemberQualifyingEvent<'info> {
    #[account(
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

    /// Must be authority or the member themselves with documentation
    pub setter: Signer<'info>,
}

pub fn set_member_qualifying_event(
    ctx: Context<SetMemberQualifyingEvent>,
    event_type: QualifyingEvent,
) -> Result<()> {
    let clock = Clock::get()?;
    let member = &mut ctx.accounts.member_account;

    // Qualifying events typically valid for 60 days
    let expires_at = clock.unix_timestamp + (60 * 24 * 60 * 60);

    member.has_qualifying_event = true;
    member.qualifying_event_at = clock.unix_timestamp;

    emit!(QualifyingEventSet {
        member: member.member,
        event_type,
        expires_at,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
