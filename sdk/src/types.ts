/**
 * Apollo Care Protocol - TypeScript Types
 * 
 * Complete type definitions for all Apollo Care Protocol accounts and instructions.
 */

import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';

// ============================================================================
// CORE TYPES
// ============================================================================

/** Basis points type (0-10000 = 0-100%) */
export type BasisPoints = number;

/** USDC amount in micro-units (6 decimals) */
export type UsdcAmount = BN;

/** APH amount in lamports (9 decimals) */
export type AphAmount = BN;

/** Unix timestamp in seconds */
export type Timestamp = number;

// ============================================================================
// MEMBERSHIP TYPES
// ============================================================================

/** Member account status */
export enum MemberStatus {
  Active = 'Active',
  Suspended = 'Suspended',
  Lapsed = 'Lapsed',
  Terminated = 'Terminated',
  Pending = 'Pending',
}

/** Coverage tier based on ACA metal levels */
export enum CoverageTier {
  Bronze = 'Bronze',     // 60% actuarial value
  Silver = 'Silver',     // 70% actuarial value
  Gold = 'Gold',         // 80% actuarial value
  Platinum = 'Platinum', // 90% actuarial value
}

/** Claim status */
export enum ClaimStatus {
  Submitted = 'Submitted',
  FastLaneApproved = 'FastLaneApproved',
  AiReview = 'AiReview',
  CommitteeReview = 'CommitteeReview',
  Approved = 'Approved',
  PartiallyApproved = 'PartiallyApproved',
  Denied = 'Denied',
  Appealed = 'Appealed',
  Paid = 'Paid',
}

export enum ClaimType {
  Medical = 'Medical',
  Dental = 'Dental',
  Vision = 'Vision',
  Pharmacy = 'Pharmacy',
  Mental = 'Mental',
  Preventive = 'Preventive',
}

export enum StakingTier {
  Conservative = 'Conservative',
  Standard = 'Standard',
  Aggressive = 'Aggressive',
}

export enum CarZone {
  Green = 'Green',
  Yellow = 'Yellow',
  Orange = 'Orange',
  Red = 'Red',
}

export enum ReserveTier {
  Tier0 = 'Tier0',
  Tier1 = 'Tier1',
  Tier2 = 'Tier2',
}

export enum ProposalStatus {
  Draft = 'Draft',
  Active = 'Active',
  Passed = 'Passed',
  Rejected = 'Rejected',
  Executed = 'Executed',
  Cancelled = 'Cancelled',
}

export enum AiRecommendation {
  AutoApprove = 'AutoApprove',
  ApproveWithReview = 'ApproveWithReview',
  RequiresCommittee = 'RequiresCommittee',
  Deny = 'Deny',
  FraudAlert = 'FraudAlert',
}

// ============================================================================
// ACCOUNT TYPES
// ============================================================================

export interface MemberAccount {
  authority: PublicKey;
  memberId: BN;
  coverageTier: CoverageTier;
  status: MemberStatus;
  enrolledAt: number;
  lastContribution: number;
  monthlyContribution: BN;
  ytdClaims: BN;
  ytdDeductible: BN;
  ytdOop: BN;
  ageAtEnrollment: number;
  regionCode: number;
  tobaccoUser: boolean;
  dependentCount: number;
  bump: number;
}

export interface ClaimAccount {
  claimId: BN;
  member: PublicKey;
  claimType: ClaimType;
  status: ClaimStatus;
  amountRequested: BN;
  amountApproved: BN;
  amountPaid: BN;
  submittedAt: number;
  resolvedAt: number | null;
  aiConfidence: number | null;
  aiFraudScore: number | null;
  aiRecommendation: AiRecommendation | null;
  bump: number;
}

export interface StakeAccount {
  authority: PublicKey;
  tier: StakingTier;
  stakedAmount: BN;
  stakedAt: number;
  unlockAt: number;
  rewardsEarned: BN;
  rewardsClaimed: BN;
  amountSlashed: BN;
  votingPower: BN;
  isLocked: boolean;
  bump: number;
}

export interface ReserveState {
  tier0Balance: BN;
  tier1Balance: BN;
  tier2Balance: BN;
  ibnrEstimate: BN;
  avgDailyClaims: BN;
  lastIbnrUpdate: number;
  bump: number;
}

export interface RiskEngineState {
  currentCar: number;
  currentZone: CarZone;
  shockFactor: number;
  expectedAnnualClaims: BN;
  eligibleStakedValue: BN;
  monthlyEnrollmentCap: number;
  lastCarUpdate: number;
  bump: number;
}

// ============================================================================
// PARAMETER TYPES
// ============================================================================

export interface EnrollmentParams {
  coverageTier: CoverageTier;
  age: number;
  regionCode: number;
  tobaccoUser: boolean;
  dependentCount: number;
}

export interface ClaimSubmissionParams {
  claimType: ClaimType;
  amountRequested: BN;
  serviceDate: number;
  providerNpi?: string;
  procedureCodes: string[];
  diagnosisCodes: string[];
}

export interface StakeParams {
  amount: BN;
  tier: StakingTier;
}

export interface TransactionResult {
  signature: string;
  slot: number;
  blockTime: number | null;
}
