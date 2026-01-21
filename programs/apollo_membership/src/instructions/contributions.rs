// programs/apollo_membership/src/instructions/contributions.rs

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};
use crate::state::{GlobalConfig, MemberAccount, MemberStatus, ContributionLedger};
use crate::errors::MembershipError;
use crate::events::{ContributionDeposited, PersistencyDiscountApplied};

/// Deposit a contribution
#[derive(Accounts)]
pub struct DepositContribution<'info> {
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
        constraint = member_account.member == member.key() @ MembershipError::Unauthorized
    )]
    pub member_account: Account<'info, MemberAccount>,

    #[account(
        mut,
        seeds = [ContributionLedger::SEED_PREFIX, member.key().as_ref()],
        bump = contribution_ledger.bump,
    )]
    pub contribution_ledger: Account<'info, ContributionLedger>,

    /// Member's USDC token account
    #[account(
        mut,
        constraint = member_token_account.mint == global_config.usdc_mint @ MembershipError::InvalidBenefitSchedule,
        constraint = member_token_account.owner == member.key() @ MembershipError::Unauthorized
    )]
    pub member_token_account: Account<'info, TokenAccount>,

    /// Protocol's contribution receiving account
    /// NOTE: In production, this routes to reserves via CPI
    #[account(mut)]
    pub protocol_token_account: Account<'info, TokenAccount>,

    pub member: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn deposit_contribution(ctx: Context<DepositContribution>, amount: u64) -> Result<()> {
    let clock = Clock::get()?;
    let config = &mut ctx.accounts.global_config;
    let member_account = &mut ctx.accounts.member_account;
    let ledger = &mut ctx.accounts.contribution_ledger;

    require!(amount > 0, MembershipError::InsufficientContribution);

    // Transfer USDC from member to protocol
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.member_token_account.to_account_info(),
                to: ctx.accounts.protocol_token_account.to_account_info(),
                authority: ctx.accounts.member.to_account_info(),
            },
        ),
        amount,
    )?;

    // Update ledger
    ledger.total_deposits = ledger.total_deposits.saturating_add(amount);
    ledger.last_deposit_at = clock.unix_timestamp;

    // Check if this covers the amount due
    let is_on_time = clock.unix_timestamp <= ledger.next_payment_due + (7 * 24 * 60 * 60); // 7 day grace

    if amount >= ledger.amount_due {
        // Full payment
        ledger.balance = ledger.balance.saturating_add(amount - ledger.amount_due);
        ledger.total_applied = ledger.total_applied.saturating_add(ledger.amount_due);

        if is_on_time {
            ledger.on_time_payments += 1;
            member_account.consecutive_months += 1;
        } else {
            ledger.late_payments += 1;
            // Don't reset consecutive months for slightly late payment
        }

        // Set next payment due (monthly)
        ledger.next_payment_due = clock.unix_timestamp + (30 * 24 * 60 * 60);
        ledger.amount_due = member_account.monthly_contribution;
    } else {
        // Partial payment - add to balance
        ledger.balance = ledger.balance.saturating_add(amount);
    }

    // Update member totals
    member_account.total_contributions_paid = member_account.total_contributions_paid
        .saturating_add(amount);
    member_account.last_contribution_at = clock.unix_timestamp;

    // Update global totals
    config.total_contributions = config.total_contributions.saturating_add(amount);

    emit!(ContributionDeposited {
        member: ctx.accounts.member.key(),
        amount,
        total_contributions: member_account.total_contributions_paid,
        next_payment_due: ledger.next_payment_due,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Apply persistency discount
#[derive(Accounts)]
pub struct ApplyPersistencyDiscount<'info> {
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

    #[account(
        mut,
        seeds = [ContributionLedger::SEED_PREFIX, member_account.member.as_ref()],
        bump = contribution_ledger.bump,
    )]
    pub contribution_ledger: Account<'info, ContributionLedger>,
}

pub fn apply_persistency_discount(ctx: Context<ApplyPersistencyDiscount>) -> Result<()> {
    let clock = Clock::get()?;
    let config = &ctx.accounts.global_config;
    let member = &mut ctx.accounts.member_account;
    let ledger = &mut ctx.accounts.contribution_ledger;

    // Check eligibility
    require!(
        member.consecutive_months >= config.persistency_discount_start_months as u16,
        MembershipError::PersistencyNotAvailable
    );

    // Calculate discount based on years of continuous coverage
    let years = member.consecutive_months / 12;
    let discount_bps = (years as u16 * config.persistency_discount_bps)
        .min(config.max_persistency_discount_bps);

    // Already at max?
    require!(
        discount_bps > member.persistency_discount_bps,
        MembershipError::MaxPersistencyReached
    );

    member.persistency_discount_bps = discount_bps;

    // Calculate new contribution
    let base_contribution = member.monthly_contribution;
    let discount_amount = base_contribution
        .saturating_mul(discount_bps as u64)
        .checked_div(10000)
        .unwrap_or(0);
    let new_contribution = base_contribution.saturating_sub(discount_amount);

    member.monthly_contribution = new_contribution;
    ledger.amount_due = new_contribution;

    emit!(PersistencyDiscountApplied {
        member: member.member,
        consecutive_months: member.consecutive_months,
        discount_bps,
        new_contribution,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Check payment status
#[derive(Accounts)]
pub struct CheckPaymentStatus<'info> {
    #[account(
        seeds = [MemberAccount::SEED_PREFIX, member_account.member.as_ref()],
        bump = member_account.bump,
    )]
    pub member_account: Account<'info, MemberAccount>,

    #[account(
        seeds = [ContributionLedger::SEED_PREFIX, member_account.member.as_ref()],
        bump = contribution_ledger.bump,
    )]
    pub contribution_ledger: Account<'info, ContributionLedger>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct PaymentStatus {
    pub is_current: bool,
    pub days_until_due: i64,
    pub amount_due: u64,
    pub balance: u64,
    pub on_time_payments: u32,
    pub late_payments: u32,
}

pub fn check_payment_status(ctx: Context<CheckPaymentStatus>) -> Result<PaymentStatus> {
    let clock = Clock::get()?;
    let ledger = &ctx.accounts.contribution_ledger;

    let days_until_due = (ledger.next_payment_due - clock.unix_timestamp) / (24 * 60 * 60);
    let is_current = clock.unix_timestamp <= ledger.next_payment_due + (7 * 24 * 60 * 60);

    Ok(PaymentStatus {
        is_current,
        days_until_due,
        amount_due: ledger.amount_due,
        balance: ledger.balance,
        on_time_payments: ledger.on_time_payments,
        late_payments: ledger.late_payments,
    })
}
