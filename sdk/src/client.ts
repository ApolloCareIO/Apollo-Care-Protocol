/**
 * Apollo Care Protocol - SDK Client
 */

import { Connection, PublicKey, Keypair, Transaction } from '@solana/web3.js';
import { AnchorProvider, Program, BN } from '@coral-xyz/anchor';
import { TOKEN_2022_PROGRAM_ID, getAssociatedTokenAddressSync } from '@solana/spl-token';
import {
  MemberAccount, ClaimAccount, StakeAccount, ReserveState, RiskEngineState,
  EnrollmentParams, ClaimSubmissionParams, StakeParams, TransactionResult,
  CoverageTier, CarZone,
} from './types';
import { calculateContribution, getCarZone, getEnrollmentCap } from './utils';

export interface ApolloClientConfig {
  endpoint: string;
  wsEndpoint?: string;
  commitment?: 'processed' | 'confirmed' | 'finalized';
}

export class ApolloClient {
  private connection: Connection;
  private provider: AnchorProvider | null = null;
  
  constructor(config: ApolloClientConfig) {
    this.connection = new Connection(config.endpoint, config.commitment || 'confirmed');
  }

  async connect(wallet: any): Promise<void> {
    this.provider = new AnchorProvider(this.connection, wallet, { commitment: 'confirmed' });
  }

  getConnection(): Connection {
    return this.connection;
  }

  // Membership
  getMemberPda(authority: PublicKey, programId: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from('member'), authority.toBuffer()],
      programId
    );
  }

  async getMember(authority: PublicKey): Promise<MemberAccount | null> {
    // Implementation uses Anchor IDL deserialization
    return null;
  }

  calculateContribution(params: EnrollmentParams, shockFactor = 10000): BN {
    return calculateContribution(
      params.coverageTier,
      params.age,
      params.tobaccoUser,
      params.dependentCount,
      shockFactor
    );
  }

  // Claims
  getClaimPda(member: PublicKey, claimId: BN, programId: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from('claim'), member.toBuffer(), claimId.toArrayLike(Buffer, 'le', 8)],
      programId
    );
  }

  async checkFastLaneEligibility(member: PublicKey, amount: BN): Promise<{ eligible: boolean; reason?: string }> {
    const MAX_FAST_LANE = new BN(500_000000);
    if (amount.gt(MAX_FAST_LANE)) {
      return { eligible: false, reason: 'Amount exceeds $500 fast-lane limit' };
    }
    return { eligible: true };
  }

  // Staking
  getStakePda(authority: PublicKey, programId: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from('stake'), authority.toBuffer()],
      programId
    );
  }

  // Risk Engine
  async getCarStatus(): Promise<{ car: number; zone: CarZone; enrollmentCap: number; shockFactor: number }> {
    // Placeholder - would fetch from RiskEngine account
    const car = 15500; // 155%
    const zone = getCarZone(car);
    return {
      car: car / 100,
      zone,
      enrollmentCap: getEnrollmentCap(zone),
      shockFactor: 1.0,
    };
  }

  // Reserves
  async getReserves(): Promise<ReserveState | null> {
    return null;
  }

  async getReserveHealth(): Promise<{ tier0Days: number; tier1Days: number; tier2Days: number; totalReserves: BN }> {
    return {
      tier0Days: 30,
      tier1Days: 60,
      tier2Days: 180,
      totalReserves: new BN(0),
    };
  }
}

export function createMainnetClient(): ApolloClient {
  return new ApolloClient({ endpoint: 'https://api.mainnet-beta.solana.com', commitment: 'confirmed' });
}

export function createDevnetClient(): ApolloClient {
  return new ApolloClient({ endpoint: 'https://api.devnet.solana.com', commitment: 'confirmed' });
}

export function createLocalClient(): ApolloClient {
  return new ApolloClient({ endpoint: 'http://localhost:8899', commitment: 'confirmed' });
}
