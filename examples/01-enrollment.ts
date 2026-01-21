/**
 * Apollo Care Protocol - Enrollment Example
 * 
 * This example demonstrates how to:
 * 1. Connect to the Solana network
 * 2. Check enrollment eligibility
 * 3. Calculate monthly contribution
 * 4. Complete enrollment
 */

import { Connection, Keypair, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { ApolloClient, CoverageTier, NETWORKS } from '@apollocare/sdk';

async function main() {
  // 1. Setup connection and wallet
  console.log('🔗 Connecting to Solana devnet...');
  const connection = new Connection(NETWORKS.DEVNET.url, 'confirmed');
  
  // In production, use a real wallet adapter
  const wallet = Keypair.generate();
  console.log(`📝 Wallet: ${wallet.publicKey.toBase58()}`);
  
  // Airdrop SOL for testing (devnet only)
  const airdropSig = await connection.requestAirdrop(
    wallet.publicKey,
    2 * LAMPORTS_PER_SOL
  );
  await connection.confirmTransaction(airdropSig);
  console.log('💰 Airdropped 2 SOL');

  // 2. Initialize Apollo client
  const client = new ApolloClient({
    endpoint: NETWORKS.DEVNET.url,
    wallet: new Wallet(wallet),
    commitment: 'confirmed',
  });

  // 3. Check if enrollment is open
  const isOpen = await client.isEnrollmentOpen();
  console.log(`\n📊 Enrollment Status: ${isOpen ? 'OPEN' : 'CLOSED'}`);
  
  if (!isOpen) {
    console.log('❌ Enrollment is currently frozen. Try again later.');
    return;
  }

  // 4. Define enrollment parameters
  const enrollmentParams = {
    age: 35,
    tobaccoUser: false,
    dependentCount: 2,        // Spouse + 1 child
    coverageTier: CoverageTier.Gold,
    state: 'WY',              // Wyoming (DAO domicile state)
    zipCode: '82001',
  };

  console.log('\n📋 Enrollment Parameters:');
  console.log(`   Age: ${enrollmentParams.age}`);
  console.log(`   Tobacco: ${enrollmentParams.tobaccoUser ? 'Yes' : 'No'}`);
  console.log(`   Dependents: ${enrollmentParams.dependentCount}`);
  console.log(`   Coverage Tier: ${CoverageTier[enrollmentParams.coverageTier]}`);

  // 5. Calculate monthly contribution
  const contribution = await client.calculateContribution(enrollmentParams);
  const contributionUsd = Number(contribution) / 1_000_000; // USDC has 6 decimals
  
  console.log(`\n💵 Monthly Contribution: $${contributionUsd.toFixed(2)} USDC`);

  // 6. Complete enrollment (would require USDC tokens in production)
  console.log('\n🚀 Submitting enrollment...');
  
  try {
    const result = await client.enroll(enrollmentParams);
    console.log(`✅ Enrollment successful!`);
    console.log(`   Transaction: ${result.signature}`);
  } catch (error) {
    console.error('❌ Enrollment failed:', error);
    console.log('\nNote: In production, ensure you have sufficient USDC for contribution.');
  }

  // 7. Verify enrollment
  const member = await client.getMember(wallet.publicKey);
  if (member) {
    console.log('\n👤 Member Account Created:');
    console.log(`   Member ID: ${member.memberId}`);
    console.log(`   Status: ${member.status}`);
    console.log(`   Tier: ${CoverageTier[member.coverageTier]}`);
    console.log(`   Monthly Contribution: $${Number(member.monthlyContribution) / 1_000_000}`);
  }
}

main()
  .then(() => process.exit(0))
  .catch((err) => {
    console.error(err);
    process.exit(1);
  });
