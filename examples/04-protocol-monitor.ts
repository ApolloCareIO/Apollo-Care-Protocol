/**
 * Apollo Care Protocol - Protocol Monitor Example
 * 
 * This example demonstrates how to:
 * 1. Monitor protocol health (CAR, reserves, risk metrics)
 * 2. Track reserve tier balances
 * 3. Subscribe to protocol events
 * 4. Build a real-time dashboard
 */

import { Connection, PublicKey } from '@solana/web3.js';
import {
  ApolloClient,
  CarZone,
  ReserveTier,
  ApolloEventType,
  NETWORKS,
  CAR_ZONES,
  RESERVE_DAYS,
} from '@apollocare/sdk';

// Simplified client for read-only operations
function createReadOnlyClient(endpoint: string): ApolloClient {
  return new ApolloClient({
    endpoint,
    wallet: {
      publicKey: PublicKey.default,
      signTransaction: async () => { throw new Error('Read-only'); },
      signAllTransactions: async () => { throw new Error('Read-only'); },
    },
    commitment: 'confirmed',
  });
}

async function main() {
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('           APOLLO CARE PROTOCOL - HEALTH MONITOR');
  console.log('═══════════════════════════════════════════════════════════════\n');

  const client = createReadOnlyClient(NETWORKS.DEVNET.url);

  // ==========================================================================
  // Fetch Protocol State
  // ==========================================================================
  const [reserves, riskEngine] = await Promise.all([
    client.getReserveState(),
    client.getRiskEngineState(),
  ]);

  if (!reserves || !riskEngine) {
    console.log('❌ Unable to fetch protocol state. Is the protocol deployed?');
    return;
  }

  // ==========================================================================
  // Capital Adequacy Ratio (CAR)
  // ==========================================================================
  const carPct = riskEngine.capitalAdequacyRatio / 100;
  const carZone = riskEngine.currentZone;
  
  console.log('📊 CAPITAL ADEQUACY RATIO');
  console.log('─────────────────────────────────────────────────────────────────');
  
  // Visual CAR meter
  const carBar = createBar(Math.min(carPct, 200), 200);
  console.log(`   [${carBar}] ${carPct.toFixed(1)}%`);
  console.log('');
  
  // Zone indicator
  const zoneColors: Record<CarZone, string> = {
    [CarZone.Green]: '🟢 GREEN',
    [CarZone.Yellow]: '🟡 YELLOW',
    [CarZone.Orange]: '🟠 ORANGE',
    [CarZone.Red]: '🔴 RED',
  };
  
  console.log(`   Current Zone: ${zoneColors[carZone]}`);
  console.log(`   Enrollment: ${getEnrollmentStatus(carZone)}`);
  console.log('');
  
  // Zone thresholds
  console.log('   Zone Thresholds:');
  console.log(`   🟢 Green:  ≥150% (Unlimited enrollment)`);
  console.log(`   🟡 Yellow: 125-150% (Max 500/month)`);
  console.log(`   🟠 Orange: 100-125% (Max 100/month)`);
  console.log(`   🔴 Red:    <100% (Enrollment frozen)`);
  console.log('');

  // ==========================================================================
  // Reserve Balances
  // ==========================================================================
  console.log('💰 RESERVE TIERS');
  console.log('─────────────────────────────────────────────────────────────────');
  
  const tier0Usd = Number(reserves.tier0Balance) / 1_000_000;
  const tier1Usd = Number(reserves.tier1Balance) / 1_000_000;
  const tier2Usd = Number(reserves.tier2Balance) / 1_000_000;
  const totalUsd = tier0Usd + tier1Usd + tier2Usd;
  
  const avgDailyClaims = Number(reserves.avgDailyClaims) / 1_000_000;
  
  console.log(`   Tier 0 (Liquidity):    $${formatNumber(tier0Usd)}`);
  console.log(`      Target: ${RESERVE_DAYS.TIER0} days = $${formatNumber(avgDailyClaims * RESERVE_DAYS.TIER0)}`);
  console.log(`      Current: ${(tier0Usd / avgDailyClaims).toFixed(1)} days`);
  console.log('');
  
  console.log(`   Tier 1 (Operating):    $${formatNumber(tier1Usd)}`);
  console.log(`      Target: ${RESERVE_DAYS.TIER1} days = $${formatNumber(avgDailyClaims * RESERVE_DAYS.TIER1)}`);
  console.log(`      Current: ${(tier1Usd / avgDailyClaims).toFixed(1)} days`);
  console.log(`      IBNR Reserve: $${formatNumber(Number(reserves.ibnrReserve) / 1_000_000)}`);
  console.log('');
  
  console.log(`   Tier 2 (Contingent):   $${formatNumber(tier2Usd)}`);
  console.log(`      Target: ${RESERVE_DAYS.TIER2} days = $${formatNumber(avgDailyClaims * RESERVE_DAYS.TIER2)}`);
  console.log(`      Current: ${(tier2Usd / avgDailyClaims).toFixed(1)} days`);
  console.log('');
  
  console.log(`   TOTAL RESERVES:        $${formatNumber(totalUsd)}`);
  console.log('');

  // ==========================================================================
  // Risk Metrics
  // ==========================================================================
  console.log('⚠️  RISK METRICS');
  console.log('─────────────────────────────────────────────────────────────────');
  
  const shockFactor = riskEngine.shockFactor / 10000;
  console.log(`   Shock Factor: ${shockFactor.toFixed(2)}x`);
  console.log(`      Range: 1.0x (normal) → 2.0x (stressed)`);
  console.log(`      Premium Impact: ${((shockFactor - 1) * 100).toFixed(0)}% surcharge`);
  console.log('');
  
  // Medical Loss Ratio
  if (reserves.totalPremiumsCollected && Number(reserves.totalPremiumsCollected) > 0) {
    const mlr = (Number(reserves.totalClaimsPaid) / Number(reserves.totalPremiumsCollected)) * 100;
    console.log(`   Medical Loss Ratio: ${mlr.toFixed(1)}%`);
    console.log(`      Target: ≥90% (vs traditional insurers ~82%)`);
    console.log(`      ${mlr >= 90 ? '✅ On target' : '⚠️ Below target'}`);
    console.log('');
  }

  // ==========================================================================
  // Claims Statistics
  // ==========================================================================
  console.log('📋 CLAIMS STATISTICS');
  console.log('─────────────────────────────────────────────────────────────────');
  
  console.log(`   Average Daily Claims: $${formatNumber(avgDailyClaims)}`);
  console.log(`   Total Claims Paid: $${formatNumber(Number(reserves.totalClaimsPaid) / 1_000_000)}`);
  console.log('');

  // ==========================================================================
  // Health Score
  // ==========================================================================
  console.log('🏥 PROTOCOL HEALTH SCORE');
  console.log('─────────────────────────────────────────────────────────────────');
  
  const healthScore = calculateHealthScore(
    carPct,
    tier0Usd / avgDailyClaims,
    tier1Usd / avgDailyClaims,
    tier2Usd / avgDailyClaims,
    shockFactor
  );
  
  const healthBar = createBar(healthScore, 100);
  console.log(`   [${healthBar}] ${healthScore.toFixed(0)}/100`);
  console.log(`   Status: ${getHealthStatus(healthScore)}`);
  console.log('');

  // ==========================================================================
  // Event Subscription Example
  // ==========================================================================
  console.log('📡 EVENT SUBSCRIPTION');
  console.log('─────────────────────────────────────────────────────────────────');
  console.log('   Available event types for real-time monitoring:');
  console.log('');
  Object.values(ApolloEventType).forEach(event => {
    console.log(`   • ${event}`);
  });
  console.log('');
  console.log('   Example subscription code:');
  console.log('   ```');
  console.log('   connection.onLogs(CLAIMS_PROGRAM_ID, (logs) => {');
  console.log('     // Parse and handle claim events');
  console.log('   });');
  console.log('   ```');
  console.log('');

  console.log('═══════════════════════════════════════════════════════════════');
  console.log(`   Last Updated: ${new Date().toISOString()}`);
  console.log('═══════════════════════════════════════════════════════════════\n');
}

// ==========================================================================
// Helper Functions
// ==========================================================================

function createBar(value: number, max: number, width: number = 40): string {
  const filled = Math.round((value / max) * width);
  const empty = width - filled;
  return '█'.repeat(Math.max(0, filled)) + '░'.repeat(Math.max(0, empty));
}

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toFixed(2);
}

function getEnrollmentStatus(zone: CarZone): string {
  switch (zone) {
    case CarZone.Green: return 'OPEN (Unlimited)';
    case CarZone.Yellow: return 'LIMITED (500/month max)';
    case CarZone.Orange: return 'RESTRICTED (100/month max)';
    case CarZone.Red: return '❌ FROZEN';
  }
}

function calculateHealthScore(
  carPct: number,
  tier0Days: number,
  tier1Days: number,
  tier2Days: number,
  shockFactor: number
): number {
  let score = 0;
  
  // CAR contribution (40 points max)
  if (carPct >= 150) score += 40;
  else if (carPct >= 125) score += 30;
  else if (carPct >= 100) score += 20;
  else score += (carPct / 100) * 15;
  
  // Reserve adequacy (40 points max)
  const tier0Score = Math.min(tier0Days / 30, 1) * 10;
  const tier1Score = Math.min(tier1Days / 60, 1) * 15;
  const tier2Score = Math.min(tier2Days / 180, 1) * 15;
  score += tier0Score + tier1Score + tier2Score;
  
  // Shock factor (20 points max - lower is better)
  const shockScore = Math.max(0, 20 - ((shockFactor - 1) * 20));
  score += shockScore;
  
  return Math.min(100, score);
}

function getHealthStatus(score: number): string {
  if (score >= 90) return '🟢 EXCELLENT';
  if (score >= 75) return '🟢 GOOD';
  if (score >= 60) return '🟡 FAIR';
  if (score >= 40) return '🟠 CONCERNING';
  return '🔴 CRITICAL';
}

main()
  .then(() => process.exit(0))
  .catch((err) => {
    console.error(err);
    process.exit(1);
  });
