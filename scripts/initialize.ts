/**
 * Apollo Care Protocol - Initialization Script
 * 
 * This script initializes all protocol programs with proper parameters.
 * Run after deployment: npx ts-node scripts/initialize.ts
 */

import * as anchor from '@coral-xyz/anchor';
import { Program, AnchorProvider, BN } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey, SystemProgram } from '@solana/web3.js';
import { TOKEN_2022_PROGRAM_ID } from '@solana/spl-token';
import * as fs from 'fs';
import * as path from 'path';

// Configuration
const CLUSTER = process.env.CLUSTER || 'devnet';
const WALLET_PATH = process.env.WALLET_PATH || `${process.env.HOME}/.config/solana/id.json`;

// APH Token (Token-2022)
const APH_MINT = new PublicKey('6S3T6f1mmhxQWKR1gMiZR1SZLpu396jnnRMGqAZUj3Qj');

// Protocol Parameters
const PROTOCOL_PARAMS = {
  // CAR Zones (basis points)
  carGreenThreshold: 15000,   // 150%
  carYellowThreshold: 12500,  // 125%
  carOrangeThreshold: 10000,  // 100%
  
  // Reserve Targets (days)
  tier0TargetDays: 30,
  tier1TargetDays: 60,
  tier2TargetDays: 180,
  
  // MLR Target (basis points)
  targetMlr: 9000,  // 90%
  
  // Shock Factor Range
  shockFactorMin: 10000,  // 1.0x
  shockFactorMax: 20000,  // 2.0x
  
  // Staking Tiers
  conservativeLockDays: 30,
  standardLockDays: 90,
  aggressiveLockDays: 180,
  
  // Fast-lane limits
  fastLaneMaxAmount: new BN(500_000000),  // $500 USDC
  fastLaneMaxPer30Days: 5,
  
  // Reinsurance defaults (bootstrap)
  specificAttachment: new BN(50000_000000),  // $50k
  specificCoverageBps: 9000,                  // 90%
  aggregateTriggerBps: 11000,                 // 110%
  aggregateCoverageBps: 10000,                // 100%
  aggregateCapBps: 15000,                     // 150%
  
  // Governance
  defaultQuorumBps: 1000,      // 10%
  defaultPassThreshold: 5001,  // 50% + 1
  minVotingPeriod: 3 * 24 * 60 * 60,   // 3 days
  maxVotingPeriod: 14 * 24 * 60 * 60,  // 14 days
  executionDelay: 24 * 60 * 60,        // 24 hours
};

async function main() {
  console.log('🚀 Apollo Care Protocol - Initialization');
  console.log('=========================================');
  console.log(`Cluster: ${CLUSTER}`);
  console.log('');
  
  // Load wallet
  const walletKeypair = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(WALLET_PATH, 'utf-8')))
  );
  console.log(`Wallet: ${walletKeypair.publicKey.toBase58()}`);
  
  // Setup connection
  const endpoint = CLUSTER === 'mainnet-beta' 
    ? 'https://api.mainnet-beta.solana.com'
    : CLUSTER === 'devnet'
    ? 'https://api.devnet.solana.com'
    : 'http://localhost:8899';
  
  const connection = new Connection(endpoint, 'confirmed');
  
  // Setup provider
  const wallet = new anchor.Wallet(walletKeypair);
  const provider = new AnchorProvider(connection, wallet, { commitment: 'confirmed' });
  anchor.setProvider(provider);
  
  // Load IDLs and create programs
  const programs = await loadPrograms(provider);
  
  console.log('');
  console.log('📋 Initializing Programs...');
  console.log('');
  
  // 1. Initialize Core
  console.log('1️⃣ Initializing Apollo Core...');
  await initializeCore(programs.core, walletKeypair);
  
  // 2. Initialize Risk Engine
  console.log('2️⃣ Initializing Risk Engine...');
  await initializeRiskEngine(programs.riskEngine, walletKeypair);
  
  // 3. Initialize Reserves
  console.log('3️⃣ Initializing Reserves...');
  await initializeReserves(programs.reserves, walletKeypair);
  
  // 4. Initialize Governance
  console.log('4️⃣ Initializing Governance...');
  await initializeGovernance(programs.governance, walletKeypair);
  
  // 5. Initialize Staking
  console.log('5️⃣ Initializing Staking...');
  await initializeStaking(programs.staking, walletKeypair);
  
  // 6. Initialize Membership
  console.log('6️⃣ Initializing Membership...');
  await initializeMembership(programs.membership, walletKeypair);
  
  // 7. Initialize Claims
  console.log('7️⃣ Initializing Claims...');
  await initializeClaims(programs.claims, walletKeypair);
  
  // 8. Initialize Reinsurance
  console.log('8️⃣ Initializing Reinsurance...');
  await initializeReinsurance(programs.reinsurance, walletKeypair);
  
  console.log('');
  console.log('✅ All programs initialized successfully!');
  console.log('');
  console.log('Next steps:');
  console.log('  1. Verify initialization: npx ts-node scripts/verify.ts');
  console.log('  2. Fund reserve vaults with initial capital');
  console.log('  3. Configure multi-sig for governance');
  console.log('  4. Open enrollment');
}

async function loadPrograms(provider: AnchorProvider) {
  const idlDir = path.join(__dirname, '..', 'target', 'idl');
  
  const loadProgram = (name: string) => {
    const idlPath = path.join(idlDir, `${name}.json`);
    const idl = JSON.parse(fs.readFileSync(idlPath, 'utf-8'));
    return new Program(idl, provider);
  };
  
  return {
    core: loadProgram('apollo_core'),
    governance: loadProgram('apollo_governance'),
    staking: loadProgram('apollo_staking'),
    reserves: loadProgram('apollo_reserves'),
    riskEngine: loadProgram('apollo_risk_engine'),
    membership: loadProgram('apollo_membership'),
    claims: loadProgram('apollo_claims'),
    reinsurance: loadProgram('apollo_reinsurance'),
  };
}

async function initializeCore(program: Program, authority: Keypair) {
  const [configPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('config')],
    program.programId
  );
  
  try {
    await program.methods
      .initialize({
        aphMint: APH_MINT,
        targetMlrBps: PROTOCOL_PARAMS.targetMlr,
      })
      .accounts({
        config: configPda,
        authority: authority.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();
    console.log('   ✓ Apollo Core initialized');
  } catch (e: any) {
    if (e.message?.includes('already in use')) {
      console.log('   ⚠️ Apollo Core already initialized');
    } else {
      throw e;
    }
  }
}

async function initializeRiskEngine(program: Program, authority: Keypair) {
  const [riskEnginePda] = PublicKey.findProgramAddressSync(
    [Buffer.from('risk_engine')],
    program.programId
  );
  
  try {
    await program.methods
      .initialize({
        carGreenThreshold: PROTOCOL_PARAMS.carGreenThreshold,
        carYellowThreshold: PROTOCOL_PARAMS.carYellowThreshold,
        carOrangeThreshold: PROTOCOL_PARAMS.carOrangeThreshold,
        shockFactorMin: PROTOCOL_PARAMS.shockFactorMin,
        shockFactorMax: PROTOCOL_PARAMS.shockFactorMax,
      })
      .accounts({
        riskEngine: riskEnginePda,
        authority: authority.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();
    console.log('   ✓ Risk Engine initialized');
  } catch (e: any) {
    if (e.message?.includes('already in use')) {
      console.log('   ⚠️ Risk Engine already initialized');
    } else {
      throw e;
    }
  }
}

async function initializeReserves(program: Program, authority: Keypair) {
  const [reservesPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('reserves')],
    program.programId
  );
  
  try {
    await program.methods
      .initialize({
        tier0TargetDays: PROTOCOL_PARAMS.tier0TargetDays,
        tier1TargetDays: PROTOCOL_PARAMS.tier1TargetDays,
        tier2TargetDays: PROTOCOL_PARAMS.tier2TargetDays,
      })
      .accounts({
        reserves: reservesPda,
        authority: authority.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();
    console.log('   ✓ Reserves initialized');
  } catch (e: any) {
    if (e.message?.includes('already in use')) {
      console.log('   ⚠️ Reserves already initialized');
    } else {
      throw e;
    }
  }
}

async function initializeGovernance(program: Program, authority: Keypair) {
  const [governancePda] = PublicKey.findProgramAddressSync(
    [Buffer.from('governance')],
    program.programId
  );
  
  try {
    await program.methods
      .initialize({
        defaultQuorumBps: PROTOCOL_PARAMS.defaultQuorumBps,
        defaultPassThreshold: PROTOCOL_PARAMS.defaultPassThreshold,
        minVotingPeriod: new BN(PROTOCOL_PARAMS.minVotingPeriod),
        maxVotingPeriod: new BN(PROTOCOL_PARAMS.maxVotingPeriod),
        executionDelay: new BN(PROTOCOL_PARAMS.executionDelay),
      })
      .accounts({
        governance: governancePda,
        authority: authority.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();
    console.log('   ✓ Governance initialized');
  } catch (e: any) {
    if (e.message?.includes('already in use')) {
      console.log('   ⚠️ Governance already initialized');
    } else {
      throw e;
    }
  }
}

async function initializeStaking(program: Program, authority: Keypair) {
  const [stakingPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('staking_config')],
    program.programId
  );
  
  try {
    await program.methods
      .initialize({
        aphMint: APH_MINT,
        conservativeLockDays: PROTOCOL_PARAMS.conservativeLockDays,
        standardLockDays: PROTOCOL_PARAMS.standardLockDays,
        aggressiveLockDays: PROTOCOL_PARAMS.aggressiveLockDays,
      })
      .accounts({
        stakingConfig: stakingPda,
        authority: authority.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();
    console.log('   ✓ Staking initialized');
  } catch (e: any) {
    if (e.message?.includes('already in use')) {
      console.log('   ⚠️ Staking already initialized');
    } else {
      throw e;
    }
  }
}

async function initializeMembership(program: Program, authority: Keypair) {
  const [membershipPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('membership_config')],
    program.programId
  );
  
  try {
    await program.methods
      .initialize({})
      .accounts({
        membershipConfig: membershipPda,
        authority: authority.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();
    console.log('   ✓ Membership initialized');
  } catch (e: any) {
    if (e.message?.includes('already in use')) {
      console.log('   ⚠️ Membership already initialized');
    } else {
      throw e;
    }
  }
}

async function initializeClaims(program: Program, authority: Keypair) {
  const [claimsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('claims_config')],
    program.programId
  );
  
  try {
    await program.methods
      .initialize({
        fastLaneMaxAmount: PROTOCOL_PARAMS.fastLaneMaxAmount,
        fastLaneMaxPer30Days: PROTOCOL_PARAMS.fastLaneMaxPer30Days,
      })
      .accounts({
        claimsConfig: claimsPda,
        authority: authority.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();
    console.log('   ✓ Claims initialized');
  } catch (e: any) {
    if (e.message?.includes('already in use')) {
      console.log('   ⚠️ Claims already initialized');
    } else {
      throw e;
    }
  }
}

async function initializeReinsurance(program: Program, authority: Keypair) {
  const [reinsurancePda] = PublicKey.findProgramAddressSync(
    [Buffer.from('reinsurance_config')],
    program.programId
  );
  
  try {
    await program.methods
      .initialize({
        specificAttachment: PROTOCOL_PARAMS.specificAttachment,
        specificCoverageBps: PROTOCOL_PARAMS.specificCoverageBps,
        aggregateTriggerBps: PROTOCOL_PARAMS.aggregateTriggerBps,
        aggregateCoverageBps: PROTOCOL_PARAMS.aggregateCoverageBps,
        aggregateCapBps: PROTOCOL_PARAMS.aggregateCapBps,
      })
      .accounts({
        reinsuranceConfig: reinsurancePda,
        authority: authority.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();
    console.log('   ✓ Reinsurance initialized');
  } catch (e: any) {
    if (e.message?.includes('already in use')) {
      console.log('   ⚠️ Reinsurance already initialized');
    } else {
      throw e;
    }
  }
}

main().catch(console.error);
