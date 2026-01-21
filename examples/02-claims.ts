/**
 * Apollo Care Protocol - Claims Example
 * 
 * This example demonstrates how to:
 * 1. Submit a claim
 * 2. Check claim status
 * 3. Track claim through processing tiers
 * 4. Appeal a denied claim
 */

import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { AnchorProvider, Wallet, BN } from '@coral-xyz/anchor';
import {
  ApolloClient,
  ClaimType,
  ClaimStatus,
  ClaimSubmissionParams,
  NETWORKS,
} from '@apollocare/sdk';

async function main() {
  // Setup (in production, use wallet adapter)
  const connection = new Connection(NETWORKS.DEVNET.url, 'confirmed');
  const wallet = Keypair.generate();
  
  const client = new ApolloClient({
    endpoint: NETWORKS.DEVNET.url,
    wallet: new Wallet(wallet),
    commitment: 'confirmed',
  });

  // Verify member is enrolled
  const member = await client.getMember(wallet.publicKey);
  if (!member || member.status !== 'Active') {
    console.error('❌ Must be an active member to submit claims');
    console.log('   Run the enrollment example first.');
    return;
  }

  console.log('👤 Active member found:', member.memberId);

  // ==========================================================================
  // Example 1: Fast-Lane Eligible Claim (<$500)
  // ==========================================================================
  console.log('\n📝 Submitting Fast-Lane Eligible Claim...');

  const fastLaneClaim: ClaimSubmissionParams = {
    amount: new BN(350_000_000), // $350 USDC
    claimType: ClaimType.Preventive,
    serviceDate: new Date('2025-01-15'),
    providerId: 'NPI1234567890',
    providerName: 'Primary Care Associates',
    diagnosisCodes: ['Z00.00'], // Annual wellness visit
    procedureCodes: ['99395'],  // Preventive visit, 18-39
    documentHash: 'QmXyz123...', // IPFS hash of claim documents
    description: 'Annual wellness exam',
  };

  try {
    const result = await client.submitClaim(fastLaneClaim);
    console.log('✅ Claim submitted:', result.signature);
    
    // Fast-lane claims are processed immediately
    // Check status after a few seconds
    await new Promise(r => setTimeout(r, 5000));
    
    // Note: In production, you'd store the claim ID from the result
    // This is simplified for the example
  } catch (error) {
    console.error('❌ Claim submission failed:', error);
  }

  // ==========================================================================
  // Example 2: AI Review Claim ($500 - $5,000)
  // ==========================================================================
  console.log('\n📝 Submitting AI Review Claim...');

  const aiReviewClaim: ClaimSubmissionParams = {
    amount: new BN(2_500_000_000), // $2,500 USDC
    claimType: ClaimType.Medical,
    serviceDate: new Date('2025-01-10'),
    providerId: 'NPI9876543210',
    providerName: 'City General Hospital',
    diagnosisCodes: ['S52.501A'], // Fracture of lower end of radius
    procedureCodes: ['25605', '29075'], // Closed treatment + cast
    documentHash: 'QmAbc456...',
    description: 'ER visit and cast application for wrist fracture',
  };

  try {
    const result = await client.submitClaim(aiReviewClaim);
    console.log('✅ Claim submitted for AI review:', result.signature);
    console.log('   Expected processing time: <24 hours');
  } catch (error) {
    console.error('❌ Claim submission failed:', error);
  }

  // ==========================================================================
  // Example 3: Committee Review Claim (>$5,000)
  // ==========================================================================
  console.log('\n📝 Submitting Committee Review Claim...');

  const committeeClaim: ClaimSubmissionParams = {
    amount: new BN(15_000_000_000), // $15,000 USDC
    claimType: ClaimType.Medical,
    serviceDate: new Date('2025-01-05'),
    providerId: 'NPI5555555555',
    providerName: 'University Medical Center',
    diagnosisCodes: ['K80.10'], // Gallstone with cholecystitis
    procedureCodes: ['47562'],  // Laparoscopic cholecystectomy
    documentHash: 'QmDef789...',
    description: 'Laparoscopic gallbladder surgery',
  };

  try {
    const result = await client.submitClaim(committeeClaim);
    console.log('✅ Claim submitted for committee review:', result.signature);
    console.log('   Expected processing time: 3-5 business days');
  } catch (error) {
    console.error('❌ Claim submission failed:', error);
  }

  // ==========================================================================
  // Check Claim Status
  // ==========================================================================
  console.log('\n📊 Checking claim statuses...');

  const memberClaims = await client.getMemberClaims(wallet.publicKey);
  
  for (const claim of memberClaims) {
    console.log(`\n   Claim ID: ${claim.claimId}`);
    console.log(`   Amount: $${Number(claim.amount) / 1_000_000}`);
    console.log(`   Status: ${ClaimStatus[claim.status]}`);
    console.log(`   Processing Tier: ${claim.processingTier}`);
    
    if (claim.aiDecision) {
      console.log(`   AI Confidence: ${claim.aiDecision.confidence / 100}%`);
      console.log(`   AI Recommendation: ${claim.aiDecision.recommendation}`);
    }
    
    if (claim.status === ClaimStatus.Approved) {
      console.log(`   ✅ Approved Amount: $${Number(claim.approvedAmount) / 1_000_000}`);
    }
    
    if (claim.status === ClaimStatus.Denied) {
      console.log(`   ❌ Denial Reason: ${claim.denialReason}`);
      console.log(`   Appeals Remaining: ${2 - claim.appealCount}`);
    }
  }

  // ==========================================================================
  // Appeal a Denied Claim (if applicable)
  // ==========================================================================
  const deniedClaim = memberClaims.find(c => c.status === ClaimStatus.Denied);
  
  if (deniedClaim && deniedClaim.appealCount < 2) {
    console.log('\n📢 Appealing denied claim...');
    
    try {
      const appealResult = await client.appealClaim(
        deniedClaim.claimId,
        'Submitting additional documentation demonstrating medical necessity. ' +
        'Attached physician letter and prior authorization approval.'
      );
      console.log('✅ Appeal submitted:', appealResult.signature);
      console.log('   The claim will be re-reviewed by the committee.');
    } catch (error) {
      console.error('❌ Appeal failed:', error);
    }
  }
}

main()
  .then(() => process.exit(0))
  .catch((err) => {
    console.error(err);
    process.exit(1);
  });
