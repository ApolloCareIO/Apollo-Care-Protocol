// programs/apollo_claims/src/lib.rs
//
// Apollo Claims Program
// =====================
// Manages the complete claims lifecycle:
// - Claim submission and validation
// - Fast-lane auto-approval for small routine claims
// - AI-assisted triage for medium claims
// - Committee attestation workflow
// - Approval/denial processing
// - Payment via reserve waterfall (CPI)
// - Appeals handling

use anchor_lang::prelude::*;

pub mod ai_oracle;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("J65pg6g7caJvSvfGBsuwzzYiyxR1EJePP1NGuaPqRK6C");

#[program]
pub mod apollo_claims {
    use super::*;

    // ==================== INITIALIZATION ====================

    /// Initialize claims configuration
    pub fn initialize_claims_config(
        ctx: Context<InitializeClaimsConfig>,
        params: InitializeClaimsParams,
    ) -> Result<()> {
        instructions::initialize::handler(ctx, params)
    }

    /// Set or update benefit schedule
    pub fn set_benefit_schedule(
        ctx: Context<SetBenefitSchedule>,
        params: SetBenefitScheduleParams,
    ) -> Result<()> {
        instructions::initialize::set_benefit_schedule(ctx, params)
    }

    /// Add an attestor to the registry
    pub fn add_attestor(ctx: Context<ManageAttestor>, attestor: Pubkey) -> Result<()> {
        instructions::initialize::add_attestor(ctx, attestor)
    }

    /// Remove an attestor from the registry
    pub fn remove_attestor(ctx: Context<ManageAttestor>, attestor: Pubkey) -> Result<()> {
        instructions::initialize::remove_attestor(ctx, attestor)
    }

    // ==================== CLAIM SUBMISSION ====================

    /// Submit a new claim
    pub fn submit_claim(ctx: Context<SubmitClaim>, params: SubmitClaimParams) -> Result<()> {
        instructions::submission::submit_claim(ctx, params)
    }

    /// Cancel a claim (member only, early stages)
    pub fn cancel_claim(ctx: Context<CancelClaim>) -> Result<()> {
        instructions::submission::cancel_claim(ctx)
    }

    /// Move claim to review status
    pub fn move_to_review(ctx: Context<MoveToReview>) -> Result<()> {
        instructions::submission::move_to_review(ctx)
    }

    /// Move claim to pending attestation
    pub fn move_to_pending_attestation(ctx: Context<MoveToPendingAttestation>) -> Result<()> {
        instructions::submission::move_to_pending_attestation(ctx)
    }

    // ==================== AI ORACLE (Tier 1 & 2 Processing) ====================
    // Fast-lane auto-approval + AI-assisted triage

    /// Initialize AI Oracle for claims processing
    pub fn initialize_ai_oracle(
        ctx: Context<InitializeAiOracle>,
        authorized_signers: Vec<Pubkey>,
        required_sigs: u8,
    ) -> Result<()> {
        instructions::ai_processing::initialize_ai_oracle(ctx, authorized_signers, required_sigs)
    }

    /// Submit AI decision for a claim (oracle signer only)
    pub fn submit_ai_decision(
        ctx: Context<SubmitAiDecision>,
        params: AiDecisionParams,
    ) -> Result<()> {
        instructions::ai_processing::submit_ai_decision(ctx, params)
    }

    /// Process fast-lane auto-approval (small routine claims)
    pub fn process_fast_lane(
        ctx: Context<ProcessFastLane>,
        claim_id: u64,
        month_start: i64,
    ) -> Result<()> {
        instructions::ai_processing::process_fast_lane(ctx, claim_id, month_start)
    }

    /// Mark AI decision as overturned (for accuracy tracking)
    pub fn mark_decision_overturned(
        ctx: Context<MarkDecisionOverturned>,
        claim_id: u64,
    ) -> Result<()> {
        instructions::ai_processing::mark_decision_overturned(ctx, claim_id)
    }

    // ==================== ATTESTATION (Tier 3 Processing) ====================
    // Human review for large/complex claims

    /// Attest a claim (committee member review)
    pub fn attest_claim(ctx: Context<AttestClaim>, params: AttestClaimParams) -> Result<()> {
        instructions::attestation::attest_claim(ctx, params)
    }

    // ==================== RESOLUTION ====================

    /// Approve a claim
    pub fn approve_claim(ctx: Context<ApproveClaim>, approved_amount: u64) -> Result<()> {
        instructions::resolution::approve_claim(ctx, approved_amount)
    }

    /// Deny a claim
    pub fn deny_claim(ctx: Context<DenyClaim>, reason: String) -> Result<()> {
        instructions::resolution::deny_claim(ctx, reason)
    }

    /// Pay an approved claim
    pub fn pay_claim(ctx: Context<PayClaim>) -> Result<()> {
        instructions::resolution::pay_claim(ctx)
    }

    /// Close a finalized claim
    pub fn close_claim(ctx: Context<CloseClaim>) -> Result<()> {
        instructions::resolution::close_claim(ctx)
    }

    /// Appeal a denied claim
    pub fn appeal_claim(ctx: Context<AppealClaim>) -> Result<()> {
        instructions::resolution::appeal_claim(ctx)
    }
}
