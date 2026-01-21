// tests/utils/helpers.ts
//
// Shared test utilities and helpers for Apollo Care Protocol tests
// Reduces code duplication and provides consistent test patterns

import * as anchor from "@coral-xyz/anchor";
import { Program, AnchorProvider, BN } from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram, Connection, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getAccount, Account } from "@solana/spl-token";
import { expect } from "chai";

// ==================== TYPE DEFINITIONS ====================

export interface TestAccounts {
  authority: Keypair;
  aphMint: PublicKey;
  usdcMint: PublicKey;
}

export interface MemberTestAccounts {
  member: Keypair;
  memberTokenAccount: PublicKey;
  memberUsdcAccount: PublicKey;
}

export interface StakerTestAccounts {
  staker: Keypair;
  stakerTokenAccount: PublicKey;
}

// ==================== AIRDROP HELPERS ====================

/**
 * Airdrop SOL to multiple keypairs
 */
export async function airdropToMultiple(
  connection: Connection,
  keypairs: Keypair[],
  amount: number = 10 * LAMPORTS_PER_SOL
): Promise<void> {
  for (const kp of keypairs) {
    const sig = await connection.requestAirdrop(kp.publicKey, amount);
    await connection.confirmTransaction(sig);
  }
}

/**
 * Airdrop SOL to a single keypair
 */
export async function airdropTo(
  connection: Connection,
  keypair: Keypair,
  amount: number = 10 * LAMPORTS_PER_SOL
): Promise<void> {
  const sig = await connection.requestAirdrop(keypair.publicKey, amount);
  await connection.confirmTransaction(sig);
}

// ==================== TOKEN HELPERS ====================

/**
 * Create a mint with specified decimals
 */
export async function createTestMint(
  connection: Connection,
  payer: Keypair,
  decimals: number = 9
): Promise<PublicKey> {
  return createMint(
    connection,
    payer,
    payer.publicKey,
    null,
    decimals
  );
}

/**
 * Create APH mint (9 decimals)
 */
export async function createAphMint(
  connection: Connection,
  authority: Keypair
): Promise<PublicKey> {
  return createTestMint(connection, authority, 9);
}

/**
 * Create USDC mint (6 decimals)
 */
export async function createUsdcMint(
  connection: Connection,
  authority: Keypair
): Promise<PublicKey> {
  return createTestMint(connection, authority, 6);
}

/**
 * Create token account and mint tokens to it
 */
export async function createAndFundTokenAccount(
  connection: Connection,
  payer: Keypair,
  mint: PublicKey,
  owner: PublicKey,
  amount: number,
  mintAuthority: Keypair
): Promise<PublicKey> {
  const tokenAccount = await createAccount(
    connection,
    payer,
    mint,
    owner
  );

  if (amount > 0) {
    await mintTo(
      connection,
      payer,
      mint,
      tokenAccount,
      mintAuthority,
      amount
    );
  }

  return tokenAccount;
}

/**
 * Get token balance
 */
export async function getTokenBalance(
  connection: Connection,
  tokenAccount: PublicKey
): Promise<bigint> {
  const account = await getAccount(connection, tokenAccount);
  return account.amount;
}

// ==================== PDA HELPERS ====================

/**
 * Generic PDA derivation
 */
export function derivePDA(
  seeds: (Buffer | Uint8Array)[],
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(seeds, programId);
}

/**
 * Derive staking config PDA
 */
export function deriveStakingConfig(programId: PublicKey): PublicKey {
  const [pda] = derivePDA([Buffer.from("staking_config")], programId);
  return pda;
}

/**
 * Derive staking tier PDA
 */
export function deriveStakingTier(tierId: number, programId: PublicKey): PublicKey {
  const [pda] = derivePDA(
    [Buffer.from("staking_tier"), Buffer.from([tierId])],
    programId
  );
  return pda;
}

/**
 * Derive staker account PDA
 */
export function deriveStakerAccount(staker: PublicKey, programId: PublicKey): PublicKey {
  const [pda] = derivePDA(
    [Buffer.from("staker_account"), staker.toBuffer()],
    programId
  );
  return pda;
}

/**
 * Derive stake position PDA
 */
export function deriveStakePosition(
  staker: PublicKey,
  positionId: number,
  programId: PublicKey
): PublicKey {
  const [pda] = derivePDA(
    [
      Buffer.from("stake_position"),
      staker.toBuffer(),
      new BN(positionId).toArrayLike(Buffer, "le", 8)
    ],
    programId
  );
  return pda;
}

/**
 * Derive member account PDA
 */
export function deriveMemberAccount(member: PublicKey, programId: PublicKey): PublicKey {
  const [pda] = derivePDA(
    [Buffer.from("member"), member.toBuffer()],
    programId
  );
  return pda;
}

/**
 * Derive global config PDA (membership)
 */
export function deriveGlobalConfig(programId: PublicKey): PublicKey {
  const [pda] = derivePDA([Buffer.from("global_config")], programId);
  return pda;
}

/**
 * Derive enrollment window PDA
 */
export function deriveEnrollmentWindow(windowId: number, programId: PublicKey): PublicKey {
  const [pda] = derivePDA(
    [Buffer.from("enrollment_window"), new BN(windowId).toArrayLike(Buffer, "le", 8)],
    programId
  );
  return pda;
}

/**
 * Derive contribution ledger PDA
 */
export function deriveContributionLedger(member: PublicKey, programId: PublicKey): PublicKey {
  const [pda] = derivePDA(
    [Buffer.from("contribution_ledger"), member.toBuffer()],
    programId
  );
  return pda;
}

/**
 * Derive reserve config PDA
 */
export function deriveReserveConfig(programId: PublicKey): PublicKey {
  const [pda] = derivePDA([Buffer.from("reserve_config")], programId);
  return pda;
}

/**
 * Derive reserve state PDA
 */
export function deriveReserveState(programId: PublicKey): PublicKey {
  const [pda] = derivePDA([Buffer.from("reserve_state")], programId);
  return pda;
}

/**
 * Derive vault authority PDA
 */
export function deriveVaultAuthority(programId: PublicKey): PublicKey {
  const [pda] = derivePDA([Buffer.from("vault_authority")], programId);
  return pda;
}

/**
 * Derive claims config PDA
 */
export function deriveClaimsConfig(programId: PublicKey): PublicKey {
  const [pda] = derivePDA([Buffer.from("claims_config")], programId);
  return pda;
}

/**
 * Derive claim account PDA
 */
export function deriveClaimAccount(claimId: number, programId: PublicKey): PublicKey {
  const [pda] = derivePDA(
    [Buffer.from("claim"), new BN(claimId).toArrayLike(Buffer, "le", 8)],
    programId
  );
  return pda;
}

/**
 * Derive attestation PDA
 */
export function deriveAttestation(
  claimId: number,
  attestor: PublicKey,
  programId: PublicKey
): PublicKey {
  const [pda] = derivePDA(
    [
      Buffer.from("attestation"),
      new BN(claimId).toArrayLike(Buffer, "le", 8),
      attestor.toBuffer()
    ],
    programId
  );
  return pda;
}

/**
 * Derive attestor registry PDA
 */
export function deriveAttestorRegistry(programId: PublicKey): PublicKey {
  const [pda] = derivePDA([Buffer.from("attestor_registry")], programId);
  return pda;
}

/**
 * Derive benefit schedule PDA
 */
export function deriveBenefitSchedule(programId: PublicKey): PublicKey {
  const [pda] = derivePDA([Buffer.from("benefit_schedule")], programId);
  return pda;
}

// ==================== ASSERTION HELPERS ====================

/**
 * Assert account state change
 */
export function assertStateChange<T>(
  before: T,
  after: T,
  field: keyof T,
  expectedChange: number | bigint
): void {
  const beforeVal = before[field];
  const afterVal = after[field];

  if (typeof beforeVal === 'object' && 'toNumber' in (beforeVal as any)) {
    const diff = (afterVal as any).toNumber() - (beforeVal as any).toNumber();
    expect(diff).to.equal(Number(expectedChange));
  } else if (typeof beforeVal === 'bigint') {
    expect(afterVal as bigint - (beforeVal as bigint)).to.equal(BigInt(expectedChange));
  } else {
    expect(Number(afterVal) - Number(beforeVal)).to.equal(Number(expectedChange));
  }
}

/**
 * Assert error contains expected message
 */
export async function assertError(
  promise: Promise<any>,
  expectedError: string
): Promise<void> {
  try {
    await promise;
    expect.fail(`Expected error containing "${expectedError}" but transaction succeeded`);
  } catch (e: any) {
    const errorString = e.toString();
    expect(errorString).to.include(expectedError,
      `Expected error "${expectedError}" but got: ${errorString}`);
  }
}

/**
 * Assert transaction succeeds
 */
export async function assertSuccess(promise: Promise<any>): Promise<string> {
  try {
    const tx = await promise;
    return tx;
  } catch (e: any) {
    expect.fail(`Expected success but got error: ${e.toString()}`);
    throw e; // TypeScript needs this
  }
}

// ==================== TEST DATA HELPERS ====================

/**
 * Create test member profile data
 */
export function createMemberProfile(overrides: Partial<{
  age: number;
  isTobaccoUser: boolean;
  regionCode: number;
  numChildren: number;
  numAdditionalAdults: number;
}> = {}) {
  return {
    age: overrides.age ?? 35,
    isTobaccoUser: overrides.isTobaccoUser ?? false,
    regionCode: overrides.regionCode ?? 0,
    numChildren: overrides.numChildren ?? 0,
    numAdditionalAdults: overrides.numAdditionalAdults ?? 0,
    additionalAdultAges: [],
  };
}

/**
 * Create test claim data
 */
export function createClaimData(overrides: Partial<{
  amount: number;
  category: string;
  description: string;
}> = {}) {
  return {
    amount: new BN(overrides.amount ?? 5_000_000_000), // $5,000 default
    category: overrides.category ?? "outpatientCare",
    descriptionHash: overrides.description ?? "QmTestHash123456789",
  };
}

// ==================== AMOUNT HELPERS ====================

/**
 * Convert USDC amount to lamports (6 decimals)
 */
export function usdcToLamports(amount: number): BN {
  return new BN(amount * 10 ** 6);
}

/**
 * Convert APH amount to lamports (9 decimals)
 */
export function aphToLamports(amount: number): BN {
  return new BN(amount * 10 ** 9);
}

/**
 * Convert lamports to USDC (6 decimals)
 */
export function lamportsToUsdc(lamports: BN | bigint | number): number {
  const value = typeof lamports === 'object' && 'toNumber' in lamports
    ? lamports.toNumber()
    : Number(lamports);
  return value / 10 ** 6;
}

/**
 * Convert lamports to APH (9 decimals)
 */
export function lamportsToAph(lamports: BN | bigint | number): number {
  const value = typeof lamports === 'object' && 'toNumber' in lamports
    ? lamports.toNumber()
    : Number(lamports);
  return value / 10 ** 9;
}

// ==================== TIME HELPERS ====================

/**
 * Get current timestamp in seconds
 */
export function nowSeconds(): number {
  return Math.floor(Date.now() / 1000);
}

/**
 * Get future timestamp
 */
export function futureTimestamp(daysFromNow: number): number {
  return nowSeconds() + (daysFromNow * 24 * 60 * 60);
}

/**
 * Get past timestamp
 */
export function pastTimestamp(daysAgo: number): number {
  return nowSeconds() - (daysAgo * 24 * 60 * 60);
}

// ==================== SETUP HELPERS ====================

/**
 * Setup basic test accounts (authority, APH mint, USDC mint)
 */
export async function setupTestAccounts(
  connection: Connection
): Promise<TestAccounts> {
  const authority = Keypair.generate();
  await airdropTo(connection, authority);

  const aphMint = await createAphMint(connection, authority);
  const usdcMint = await createUsdcMint(connection, authority);

  return { authority, aphMint, usdcMint };
}

/**
 * Setup a member with token accounts
 */
export async function setupMember(
  connection: Connection,
  aphMint: PublicKey,
  usdcMint: PublicKey,
  funder: Keypair,
  aphAmount: number = 0,
  usdcAmount: number = 0
): Promise<MemberTestAccounts> {
  const member = Keypair.generate();
  await airdropTo(connection, member);

  const memberTokenAccount = await createAndFundTokenAccount(
    connection,
    funder,
    aphMint,
    member.publicKey,
    aphAmount,
    funder
  );

  const memberUsdcAccount = await createAndFundTokenAccount(
    connection,
    funder,
    usdcMint,
    member.publicKey,
    usdcAmount,
    funder
  );

  return { member, memberTokenAccount, memberUsdcAccount };
}

/**
 * Setup a staker with APH tokens
 */
export async function setupStaker(
  connection: Connection,
  aphMint: PublicKey,
  funder: Keypair,
  aphAmount: number = 100_000 * 10 ** 9
): Promise<StakerTestAccounts> {
  const staker = Keypair.generate();
  await airdropTo(connection, staker);

  const stakerTokenAccount = await createAndFundTokenAccount(
    connection,
    funder,
    aphMint,
    staker.publicKey,
    aphAmount,
    funder
  );

  return { staker, stakerTokenAccount };
}

// ==================== WAIT HELPERS ====================

/**
 * Wait for a specified number of milliseconds
 */
export function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Wait for slot advancement (for time-sensitive tests)
 */
export async function waitForSlots(
  connection: Connection,
  slots: number
): Promise<void> {
  const startSlot = await connection.getSlot();
  const targetSlot = startSlot + slots;

  while (await connection.getSlot() < targetSlot) {
    await sleep(400);
  }
}
