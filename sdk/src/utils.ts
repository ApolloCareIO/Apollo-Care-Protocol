/**
 * Apollo Care Protocol - Utility Functions
 */

import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { BPS, CAR_ZONES } from './constants';
import { CarZone, CoverageTier, StakingTier } from './types';

/**
 * Convert basis points to percentage
 */
export function bpsToPercent(bps: number): number {
  return bps / 100;
}

/**
 * Convert percentage to basis points
 */
export function percentToBps(percent: number): number {
  return Math.round(percent * 100);
}

/**
 * Convert USDC amount to human readable (6 decimals)
 */
export function formatUsdc(amount: BN | bigint | number): string {
  const value = typeof amount === 'number' ? amount : Number(amount);
  return (value / 1_000_000).toLocaleString('en-US', {
    style: 'currency',
    currency: 'USD',
  });
}

/**
 * Convert APH amount to human readable (9 decimals)
 */
export function formatAph(amount: BN | bigint | number): string {
  const value = typeof amount === 'number' ? amount : Number(amount);
  return (value / 1_000_000_000).toLocaleString('en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }) + ' APH';
}

/**
 * Parse USDC amount from string (handles $ and commas)
 */
export function parseUsdc(str: string): BN {
  const cleaned = str.replace(/[$,]/g, '');
  const value = parseFloat(cleaned);
  return new BN(Math.round(value * 1_000_000));
}

/**
 * Parse APH amount from string
 */
export function parseAph(str: string): BN {
  const cleaned = str.replace(/[APH,\s]/gi, '');
  const value = parseFloat(cleaned);
  return new BN(Math.round(value * 1_000_000_000));
}

/**
 * Determine CAR zone from CAR value (basis points)
 */
export function getCarZone(carBps: number): CarZone {
  if (carBps >= CAR_ZONES.GREEN) return CarZone.Green;
  if (carBps >= CAR_ZONES.YELLOW) return CarZone.Yellow;
  if (carBps >= CAR_ZONES.ORANGE) return CarZone.Orange;
  return CarZone.Red;
}

/**
 * Get enrollment cap for a CAR zone
 */
export function getEnrollmentCap(zone: CarZone): number {
  switch (zone) {
    case CarZone.Green: return Infinity;
    case CarZone.Yellow: return 500;
    case CarZone.Orange: return 100;
    case CarZone.Red: return 0;
  }
}

/**
 * Get CMS-compliant age factor (3:1 ratio)
 */
export function getAgeFactor(age: number): number {
  const bands = [
    { max: 14, factor: 6500 },
    { max: 20, factor: 8000 },
    { max: 24, factor: 8500 },
    { max: 29, factor: 9000 },
    { max: 34, factor: 10000 },
    { max: 39, factor: 11000 },
    { max: 44, factor: 13000 },
    { max: 49, factor: 15500 },
    { max: 54, factor: 18000 },
    { max: 59, factor: 22000 },
    { max: 64, factor: 27000 },
    { max: 999, factor: 30000 },
  ];
  
  for (const band of bands) {
    if (age <= band.max) return band.factor;
  }
  return 30000;
}

/**
 * Calculate monthly contribution
 */
export function calculateContribution(
  tier: CoverageTier,
  age: number,
  tobaccoUser: boolean,
  dependentCount: number,
  shockFactor: number = 10000
): BN {
  // Base rates (USDC, 6 decimals)
  const baseRates: Record<CoverageTier, number> = {
    [CoverageTier.Bronze]: 350_000000,
    [CoverageTier.Silver]: 450_000000,
    [CoverageTier.Gold]: 550_000000,
    [CoverageTier.Platinum]: 700_000000,
  };
  
  let contribution = baseRates[tier];
  
  // Apply age factor
  contribution = Math.round(contribution * getAgeFactor(age) / BPS);
  
  // Apply tobacco surcharge (max 1.5x)
  if (tobaccoUser) {
    contribution = Math.round(contribution * 15000 / BPS);
  }
  
  // Apply dependent factor (0.4x per child, max 3)
  const children = Math.min(dependentCount, 3);
  contribution = Math.round(contribution * (BPS + children * 4000) / BPS);
  
  // Apply shock factor
  contribution = Math.round(contribution * shockFactor / BPS);
  
  return new BN(contribution);
}

/**
 * Get staking tier parameters
 */
export function getStakingTierParams(tier: StakingTier) {
  const params = {
    [StakingTier.Conservative]: { lockDays: 30, minApy: 3, maxApy: 5, maxLoss: 2 },
    [StakingTier.Standard]: { lockDays: 90, minApy: 6, maxApy: 8, maxLoss: 5 },
    [StakingTier.Aggressive]: { lockDays: 180, minApy: 10, maxApy: 15, maxLoss: 10 },
  };
  return params[tier];
}

/**
 * Calculate IBNR (Incurred But Not Reported)
 */
export function calculateIbnr(
  avgDailyClaims: BN,
  reportingLagDays: number = 21, // 21 days per actuarial spec
  developmentFactor: number = 11500
): BN {
  return avgDailyClaims
    .muln(reportingLagDays)
    .muln(developmentFactor)
    .divn(BPS);
}

/**
 * Find PDA address helper
 */
export function findPda(
  seeds: (Buffer | Uint8Array)[],
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(seeds, programId);
}

/**
 * Format timestamp to ISO string
 */
export function formatTimestamp(timestamp: number): string {
  return new Date(timestamp * 1000).toISOString();
}

/**
 * Sleep helper for rate limiting
 */
export function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}
