/**
 * Apollo Care Protocol - Staking Example
 * 
 * This example demonstrates how to:
 * 1. Check staking tiers and parameters
 * 2. Stake APH tokens
 * 3. Monitor staking rewards
 * 4. Claim rewards and unstake
 */

import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { AnchorProvider, Wallet, BN } from '@coral-xyz/anchor';
import {
  ApolloClient,
  StakingTier,
  NETWORKS,
  STAKING_TIERS,
} from '@apollocare/sdk';

async function main() {
  // Setup
  const connection = new Connection(NETWORKS.DEVNET.url, 'confirmed');
  const wallet = Keypair.generate();
  
  const client = new ApolloClient({
    endpoint: NETWORKS.DEVNET.url,
    wallet: new Wallet(wallet),
    commitment: 'confirmed',
  });

  console.log('📊 Apollo Care Staking Example\n');

  // ==========================================================================
  // Display Staking Tier Information
  // ==========================================================================
  console.log('📈 Available Staking Tiers:\n');
  
  console.log('   🟢 CONSERVATIVE (Low Risk)');
  console.log(`      APY Range: 3-5%`);
  console.log(`      Max Loss: 2%`);
  console.log(`      Lock Period: 30 days`);
  console.log(`      Best for: Passive income seekers\n`);
  
  console.log('   🟡 STANDARD (Balanced)');
  console.log(`      APY Range: 6-8%`);
  console.log(`      Max Loss: 5%`);
  console.log(`      Lock Period: 90 days`);
  console.log(`      Best for: Moderate risk tolerance\n`);
  
  console.log('   🔴 AGGRESSIVE (High Risk/Reward)');
  console.log(`      APY Range: 10-15%`);
  console.log(`      Max Loss: 10%`);
  console.log(`      Lock Period: 180 days`);
  console.log(`      Best for: Long-term believers\n`);

  // ==========================================================================
  // Check Current Protocol State
  // ==========================================================================
  const riskEngine = await client.getRiskEngineState();
  const reserves = await client.getReserveState();
  
  if (riskEngine && reserves) {
    console.log('📊 Protocol Health:');
    console.log(`   CAR Zone: ${riskEngine.currentZone}`);
    console.log(`   Shock Factor: ${riskEngine.shockFactor / 10000}x`);
    console.log(`   Total Reserves: $${Number(reserves.totalBalance) / 1_000_000}`);
    console.log('');
  }

  // ==========================================================================
  // Stake APH Tokens
  // ==========================================================================
  console.log('💰 Staking APH Tokens...\n');

  const stakeAmount = new BN(10_000_000_000_000); // 10,000 APH (9 decimals)
  const selectedTier = StakingTier.Standard;

  console.log(`   Amount: 10,000 APH`);
  console.log(`   Tier: ${StakingTier[selectedTier]}`);
  console.log(`   Lock Period: 90 days`);

  try {
    const result = await client.stake({
      amount: stakeAmount,
      tier: selectedTier,
    });
    
    console.log(`\n✅ Stake successful!`);
    console.log(`   Transaction: ${result.signature}`);
  } catch (error) {
    console.error('❌ Staking failed:', error);
    console.log('\nNote: Ensure you have APH tokens to stake.');
  }

  // ==========================================================================
  // Check Stake Account
  // ==========================================================================
  console.log('\n📋 Checking stake account...');
  
  const stake = await client.getStake(wallet.publicKey);
  
  if (stake) {
    console.log('\n   Stake Account Details:');
    console.log(`   Tier: ${StakingTier[stake.tier]}`);
    console.log(`   Total Staked: ${Number(stake.stakedAmount) / 1_000_000_000} APH`);
    console.log(`   Effective Stake: ${Number(stake.effectiveStake) / 1_000_000_000} APH`);
    console.log(`   Rewards Earned: ${Number(stake.rewardsEarned) / 1_000_000_000} APH`);
    console.log(`   Rewards Claimed: ${Number(stake.rewardsClaimed) / 1_000_000_000} APH`);
    console.log(`   Voting Power: ${Number(stake.votingPower) / 1_000_000_000}`);
    console.log(`   Lock Status: ${stake.isLocked ? 'LOCKED' : 'UNLOCKED'}`);
    
    if (stake.isLocked && stake.lockExpiry) {
      const unlockDate = new Date(Number(stake.lockExpiry) * 1000);
      console.log(`   Unlocks: ${unlockDate.toLocaleDateString()}`);
    }

    // Calculate pending rewards
    const pendingRewards = stake.rewardsEarned.sub(stake.rewardsClaimed);
    if (!pendingRewards.isZero()) {
      console.log(`\n   💎 Pending Rewards: ${Number(pendingRewards) / 1_000_000_000} APH`);
    }
  }

  // ==========================================================================
  // Claim Rewards
  // ==========================================================================
  if (stake && stake.rewardsEarned.gt(stake.rewardsClaimed)) {
    console.log('\n💰 Claiming staking rewards...');
    
    try {
      const result = await client.claimRewards();
      console.log(`✅ Rewards claimed!`);
      console.log(`   Transaction: ${result.signature}`);
    } catch (error) {
      console.error('❌ Claim failed:', error);
    }
  }

  // ==========================================================================
  // Unstake (if unlocked)
  // ==========================================================================
  if (stake && !stake.isLocked) {
    console.log('\n🔓 Stake is unlocked. Unstaking...');
    
    const unstakeAmount = stake.stakedAmount.div(new BN(2)); // Unstake 50%
    
    try {
      const result = await client.unstake(unstakeAmount);
      console.log(`✅ Unstake successful!`);
      console.log(`   Amount: ${Number(unstakeAmount) / 1_000_000_000} APH`);
      console.log(`   Transaction: ${result.signature}`);
    } catch (error) {
      console.error('❌ Unstake failed:', error);
    }
  } else if (stake?.isLocked) {
    console.log('\n🔒 Stake is still locked. Cannot unstake yet.');
  }

  // ==========================================================================
  // Staking Economics Summary
  // ==========================================================================
  console.log('\n═══════════════════════════════════════════════════════════════');
  console.log('📚 STAKING ECONOMICS SUMMARY');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('');
  console.log('How Staking Works:');
  console.log('• Staked APH provides backstop capital for claims reserves');
  console.log('• Higher tiers = more risk exposure = higher potential rewards');
  console.log('• In shortfall events, staked APH may be liquidated via TWAP');
  console.log('• Maximum liquidation: 5% of staked amount per event');
  console.log('• Slippage circuit breaker: 15% (halts liquidation)');
  console.log('');
  console.log('Voting Power:');
  console.log('• Conservative: 1x voting power');
  console.log('• Standard: 1.5x voting power');
  console.log('• Aggressive: 2x voting power');
  console.log('');
  console.log('Risk Mitigation:');
  console.log('• 24-72 hour TWAP liquidation (no sudden dumps)');
  console.log('• Tier-specific loss caps (2%, 5%, or 10%)');
  console.log('• Reinsurance protects against catastrophic scenarios');
  console.log('═══════════════════════════════════════════════════════════════\n');
}

main()
  .then(() => process.exit(0))
  .catch((err) => {
    console.error(err);
    process.exit(1);
  });
