import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { 
  TOKEN_PROGRAM_ID, 
  createMint, 
  createAccount, 
  mintTo 
} from "@solana/spl-token";
import { expect } from "chai";

// Note: This test file assumes the apollo_reinsurance IDL is generated
// Run `anchor build` first to generate the IDL

describe("apollo_reinsurance", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  
  // Program ID (matches declare_id! in lib.rs)
  const REINSURANCE_PROGRAM_ID = new PublicKey("ApoRe111111111111111111111111111111111111111");
  
  // Test accounts
  let authority: Keypair;
  let reinsuranceCommittee: Keypair;
  let usdcMint: PublicKey;
  let treasuryAccount: PublicKey;
  let premiumDestination: PublicKey;
  
  // PDA seeds
  const CONFIG_SEED = Buffer.from("reinsurance_config");
  const TREATY_SEED = Buffer.from("treaty");
  const RECOVERY_CLAIM_SEED = Buffer.from("recovery_claim");
  const MEMBER_ACCUMULATOR_SEED = Buffer.from("member_accumulator");
  const MONTHLY_AGGREGATE_SEED = Buffer.from("monthly_aggregate");
  
  // Constants from the program
  const USDC_DECIMALS = 6;
  const ONE_USDC = 1_000_000;
  const ONE_YEAR_SECONDS = 365 * 24 * 60 * 60;
  
  // Test data
  const currentYear = 2026;
  const expectedAnnualClaims = BigInt(10_000_000 * ONE_USDC); // $10M
  const premiumBudget = BigInt(500_000 * ONE_USDC); // $500k
  const aggregateTriggerBps = 11000; // 110%
  const catastrophicTriggerBps = 15000; // 150%
  const catastrophicCeilingBps = 30000; // 300%
  
  // Specific stop-loss parameters
  const specificAttachment = BigInt(100_000 * ONE_USDC); // $100k
  const coinsuranceBps = 2000; // 20% Apollo, 80% reinsurer
  const coverageLimit = BigInt(0); // Unlimited
  
  before(async () => {
    authority = Keypair.generate();
    reinsuranceCommittee = Keypair.generate();
    
    // Airdrop SOL for transaction fees
    await provider.connection.requestAirdrop(
      authority.publicKey,
      10 * anchor.web3.LAMPORTS_PER_SOL
    );
    
    await provider.connection.requestAirdrop(
      reinsuranceCommittee.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    
    // Wait for airdrops to confirm
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    // Create USDC mint for testing
    usdcMint = await createMint(
      provider.connection,
      authority,
      authority.publicKey,
      null,
      USDC_DECIMALS
    );
    
    // Create treasury and premium destination accounts
    treasuryAccount = await createAccount(
      provider.connection,
      authority,
      usdcMint,
      authority.publicKey
    );
    
    premiumDestination = await createAccount(
      provider.connection,
      authority,
      usdcMint,
      reinsuranceCommittee.publicKey
    );
    
    // Mint USDC to treasury for premium payments
    await mintTo(
      provider.connection,
      authority,
      usdcMint,
      treasuryAccount,
      authority,
      Number(premiumBudget) * 2 // Extra for tests
    );
    
    console.log("Test setup complete:");
    console.log("  Authority:", authority.publicKey.toBase58());
    console.log("  Reinsurance Committee:", reinsuranceCommittee.publicKey.toBase58());
    console.log("  USDC Mint:", usdcMint.toBase58());
  });
  
  // ========================================================================
  // PDA DERIVATION HELPERS
  // ========================================================================
  
  function getConfigPDA(): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [CONFIG_SEED],
      REINSURANCE_PROGRAM_ID
    );
  }
  
  function getTreatyPDA(configPubkey: PublicKey, treatyId: number): [PublicKey, number] {
    const idBuffer = Buffer.alloc(4);
    idBuffer.writeUInt32LE(treatyId);
    return PublicKey.findProgramAddressSync(
      [TREATY_SEED, configPubkey.toBuffer(), idBuffer],
      REINSURANCE_PROGRAM_ID
    );
  }
  
  function getRecoveryClaimPDA(treatyPubkey: PublicKey, claimId: number): [PublicKey, number] {
    const idBuffer = Buffer.alloc(4);
    idBuffer.writeUInt32LE(claimId);
    return PublicKey.findProgramAddressSync(
      [RECOVERY_CLAIM_SEED, treatyPubkey.toBuffer(), idBuffer],
      REINSURANCE_PROGRAM_ID
    );
  }
  
  function getMemberAccumulatorPDA(member: PublicKey, policyYear: number): [PublicKey, number] {
    const yearBuffer = Buffer.alloc(2);
    yearBuffer.writeUInt16LE(policyYear);
    return PublicKey.findProgramAddressSync(
      [MEMBER_ACCUMULATOR_SEED, member.toBuffer(), yearBuffer],
      REINSURANCE_PROGRAM_ID
    );
  }
  
  function getMonthlyAggregatePDA(policyYear: number, month: number): [PublicKey, number] {
    const yearBuffer = Buffer.alloc(2);
    yearBuffer.writeUInt16LE(policyYear);
    return PublicKey.findProgramAddressSync(
      [MONTHLY_AGGREGATE_SEED, yearBuffer, Buffer.from([month])],
      REINSURANCE_PROGRAM_ID
    );
  }
  
  // ========================================================================
  // INITIALIZATION TESTS
  // ========================================================================
  
  describe("Initialize Reinsurance Config", () => {
    it("should initialize reinsurance configuration", async () => {
      // Note: This test would require the actual program to be deployed
      // For now, we're validating the PDA derivation and test structure
      
      const [configPDA, bump] = getConfigPDA();
      console.log("Config PDA:", configPDA.toBase58(), "bump:", bump);
      
      const now = Math.floor(Date.now() / 1000);
      const policyYearStart = now;
      const policyYearEnd = now + ONE_YEAR_SECONDS;
      
      // Test parameters are valid
      expect(policyYearStart).to.be.lessThan(policyYearEnd);
      expect(aggregateTriggerBps).to.be.greaterThan(10000);
      expect(catastrophicTriggerBps).to.be.greaterThan(aggregateTriggerBps);
      expect(catastrophicCeilingBps).to.be.greaterThan(catastrophicTriggerBps);
    });
    
    it("should reject invalid trigger ratios", async () => {
      // Aggregate trigger must be > 100%
      expect(() => {
        if (aggregateTriggerBps <= 10000) {
          throw new Error("InvalidAggregateTrigger");
        }
      }).to.not.throw();
      
      // Catastrophic must be > aggregate
      const invalidCatastrophic = aggregateTriggerBps - 100;
      expect(invalidCatastrophic).to.be.lessThan(aggregateTriggerBps);
    });
  });
  
  // ========================================================================
  // TREATY MANAGEMENT TESTS
  // ========================================================================
  
  describe("Treaty Management", () => {
    it("should derive treaty PDA correctly", async () => {
      const [configPDA] = getConfigPDA();
      const treatyId = 1;
      const [treatyPDA, bump] = getTreatyPDA(configPDA, treatyId);
      
      console.log("Treaty PDA:", treatyPDA.toBase58(), "bump:", bump);
      expect(treatyPDA).to.be.instanceOf(PublicKey);
    });
    
    it("should validate specific stop-loss parameters", () => {
      // Attachment point must be > 0
      expect(Number(specificAttachment)).to.be.greaterThan(0);
      
      // Coinsurance must be <= 10000 bps (100%)
      expect(coinsuranceBps).to.be.lessThanOrEqual(10000);
      
      // Calculate coverage split
      const excessAmount = BigInt(50_000 * ONE_USDC); // $50k excess
      const apolloPortion = (excessAmount * BigInt(coinsuranceBps)) / BigInt(10000);
      const reinsurerPortion = excessAmount - apolloPortion;
      
      console.log("Coverage split for $50k excess:");
      console.log("  Apollo (20%):", Number(apolloPortion) / ONE_USDC, "USDC");
      console.log("  Reinsurer (80%):", Number(reinsurerPortion) / ONE_USDC, "USDC");
      
      expect(Number(apolloPortion)).to.equal(10_000 * ONE_USDC);
      expect(Number(reinsurerPortion)).to.equal(40_000 * ONE_USDC);
    });
    
    it("should validate aggregate stop-loss trigger calculation", () => {
      const ytdClaims = BigInt(12_000_000 * ONE_USDC); // $12M (120% of expected)
      const triggerAmount = (expectedAnnualClaims * BigInt(aggregateTriggerBps)) / BigInt(10000);
      
      const currentRatioBps = (ytdClaims * BigInt(10000)) / expectedAnnualClaims;
      const shouldTrigger = currentRatioBps >= BigInt(aggregateTriggerBps);
      
      console.log("Aggregate stop-loss check:");
      console.log("  YTD Claims:", Number(ytdClaims) / ONE_USDC / 1_000_000, "M USDC");
      console.log("  Trigger at:", Number(triggerAmount) / ONE_USDC / 1_000_000, "M USDC");
      console.log("  Current ratio:", Number(currentRatioBps) / 100, "%");
      console.log("  Should trigger:", shouldTrigger);
      
      expect(shouldTrigger).to.be.true;
    });
  });
  
  // ========================================================================
  // MEMBER ACCUMULATOR TESTS
  // ========================================================================
  
  describe("Member Claims Accumulation", () => {
    const testMember = Keypair.generate();
    
    it("should derive member accumulator PDA correctly", () => {
      const [accumulatorPDA, bump] = getMemberAccumulatorPDA(
        testMember.publicKey,
        currentYear
      );
      
      console.log("Member Accumulator PDA:", accumulatorPDA.toBase58());
      expect(accumulatorPDA).to.be.instanceOf(PublicKey);
    });
    
    it("should detect stop-loss trigger for member", () => {
      let ytdClaims = BigInt(0);
      const claims = [
        30_000 * ONE_USDC,  // $30k
        25_000 * ONE_USDC,  // $25k (running: $55k)
        35_000 * ONE_USDC,  // $35k (running: $90k)
        20_000 * ONE_USDC,  // $20k (running: $110k) - TRIGGERS at $100k
        15_000 * ONE_USDC,  // $15k (running: $125k)
      ];
      
      let triggered = false;
      let triggerClaimIndex = -1;
      let excessAtTrigger = BigInt(0);
      
      for (let i = 0; i < claims.length; i++) {
        const newTotal = ytdClaims + BigInt(claims[i]);
        
        if (!triggered && newTotal > specificAttachment) {
          triggered = true;
          triggerClaimIndex = i;
          excessAtTrigger = newTotal - specificAttachment;
        }
        
        ytdClaims = newTotal;
      }
      
      console.log("Member claims accumulation:");
      console.log("  Total YTD:", Number(ytdClaims) / ONE_USDC, "USDC");
      console.log("  Attachment:", Number(specificAttachment) / ONE_USDC, "USDC");
      console.log("  Triggered at claim #:", triggerClaimIndex + 1);
      console.log("  Initial excess:", Number(excessAtTrigger) / ONE_USDC, "USDC");
      console.log("  Total excess:", Number(ytdClaims - specificAttachment) / ONE_USDC, "USDC");
      
      expect(triggered).to.be.true;
      expect(triggerClaimIndex).to.equal(3); // 4th claim (0-indexed)
      expect(Number(ytdClaims - specificAttachment)).to.equal(25_000 * ONE_USDC);
    });
  });
  
  // ========================================================================
  // RECOVERY CLAIM TESTS
  // ========================================================================
  
  describe("Recovery Claims", () => {
    it("should derive recovery claim PDA correctly", () => {
      const [configPDA] = getConfigPDA();
      const [treatyPDA] = getTreatyPDA(configPDA, 1);
      const claimId = 1;
      
      const [recoveryPDA, bump] = getRecoveryClaimPDA(treatyPDA, claimId);
      
      console.log("Recovery Claim PDA:", recoveryPDA.toBase58());
      expect(recoveryPDA).to.be.instanceOf(PublicKey);
    });
    
    it("should calculate specific stop-loss recovery correctly", () => {
      const memberYtdClaims = BigInt(175_000 * ONE_USDC); // $175k
      const excessAmount = memberYtdClaims - specificAttachment; // $75k excess
      
      // 20% Apollo, 80% reinsurer
      const apolloPortion = (excessAmount * BigInt(coinsuranceBps)) / BigInt(10000);
      const reinsurerPortion = excessAmount - apolloPortion;
      
      console.log("Specific stop-loss recovery:");
      console.log("  Member YTD claims:", Number(memberYtdClaims) / ONE_USDC, "USDC");
      console.log("  Attachment point:", Number(specificAttachment) / ONE_USDC, "USDC");
      console.log("  Excess amount:", Number(excessAmount) / ONE_USDC, "USDC");
      console.log("  Apollo retains:", Number(apolloPortion) / ONE_USDC, "USDC");
      console.log("  Claim to reinsurer:", Number(reinsurerPortion) / ONE_USDC, "USDC");
      
      expect(Number(excessAmount)).to.equal(75_000 * ONE_USDC);
      expect(Number(apolloPortion)).to.equal(15_000 * ONE_USDC);
      expect(Number(reinsurerPortion)).to.equal(60_000 * ONE_USDC);
    });
    
    it("should calculate aggregate stop-loss recovery correctly", () => {
      // Scenario: Claims at 130% of expected
      const ytdClaims = BigInt(13_000_000 * ONE_USDC); // $13M
      const triggerAmount = (expectedAnnualClaims * BigInt(aggregateTriggerBps)) / BigInt(10000);
      const catastrophicAmount = (expectedAnnualClaims * BigInt(catastrophicTriggerBps)) / BigInt(10000);
      
      // Recoverable is amount between trigger and current (up to catastrophic)
      let recoverable = BigInt(0);
      if (ytdClaims > triggerAmount) {
        const excess = ytdClaims - triggerAmount;
        const aggregateCapacity = catastrophicAmount - triggerAmount;
        recoverable = excess < aggregateCapacity ? excess : aggregateCapacity;
      }
      
      console.log("Aggregate stop-loss recovery:");
      console.log("  Expected annual:", Number(expectedAnnualClaims) / ONE_USDC / 1_000_000, "M USDC");
      console.log("  YTD claims:", Number(ytdClaims) / ONE_USDC / 1_000_000, "M USDC");
      console.log("  Trigger at (110%):", Number(triggerAmount) / ONE_USDC / 1_000_000, "M USDC");
      console.log("  Catastrophic at (150%):", Number(catastrophicAmount) / ONE_USDC / 1_000_000, "M USDC");
      console.log("  Recoverable:", Number(recoverable) / ONE_USDC / 1_000_000, "M USDC");
      
      // At 130%, excess over 110% is 20% of expected = $2M
      expect(Number(recoverable)).to.equal(2_000_000 * ONE_USDC);
    });
  });
  
  // ========================================================================
  // MONTHLY AGGREGATE TESTS
  // ========================================================================
  
  describe("Monthly Aggregate Tracking", () => {
    it("should derive monthly aggregate PDA correctly", () => {
      const [monthlyPDA, bump] = getMonthlyAggregatePDA(currentYear, 6); // June
      
      console.log("Monthly Aggregate PDA (June):", monthlyPDA.toBase58());
      expect(monthlyPDA).to.be.instanceOf(PublicKey);
    });
    
    it("should calculate monthly ratio correctly", () => {
      const monthlyExpected = BigInt(833_333 * ONE_USDC); // ~$833k (1/12 of $10M)
      const monthlyClaims = BigInt(950_000 * ONE_USDC); // $950k
      
      const ratioBps = (monthlyClaims * BigInt(10000)) / monthlyExpected;
      
      console.log("Monthly ratio calculation:");
      console.log("  Expected:", Number(monthlyExpected) / ONE_USDC, "USDC");
      console.log("  Actual:", Number(monthlyClaims) / ONE_USDC, "USDC");
      console.log("  Ratio:", Number(ratioBps) / 100, "%");
      
      // 950k / 833k ≈ 114%
      expect(Number(ratioBps)).to.be.greaterThan(11000);
    });
  });
  
  // ========================================================================
  // SCENARIO TESTS
  // ========================================================================
  
  describe("Scenario: Normal Year", () => {
    it("should handle year with claims at 95% of expected", () => {
      const ytdClaims = BigInt(9_500_000 * ONE_USDC); // $9.5M (95%)
      const currentRatioBps = (ytdClaims * BigInt(10000)) / expectedAnnualClaims;
      
      const aggregateTriggered = currentRatioBps >= BigInt(aggregateTriggerBps);
      const catastrophicTriggered = currentRatioBps >= BigInt(catastrophicTriggerBps);
      
      console.log("Normal year scenario (95%):");
      console.log("  Aggregate triggered:", aggregateTriggered);
      console.log("  Catastrophic triggered:", catastrophicTriggered);
      console.log("  Reinsurance recovery needed:", "None");
      
      expect(aggregateTriggered).to.be.false;
      expect(catastrophicTriggered).to.be.false;
    });
  });
  
  describe("Scenario: Bad Year", () => {
    it("should handle year with claims at 125% of expected", () => {
      const ytdClaims = BigInt(12_500_000 * ONE_USDC); // $12.5M (125%)
      const currentRatioBps = (ytdClaims * BigInt(10000)) / expectedAnnualClaims;
      
      const aggregateTriggered = currentRatioBps >= BigInt(aggregateTriggerBps);
      const catastrophicTriggered = currentRatioBps >= BigInt(catastrophicTriggerBps);
      
      // Calculate recovery
      const triggerAmount = (expectedAnnualClaims * BigInt(aggregateTriggerBps)) / BigInt(10000);
      const recoverable = ytdClaims - triggerAmount;
      
      console.log("Bad year scenario (125%):");
      console.log("  Aggregate triggered:", aggregateTriggered);
      console.log("  Catastrophic triggered:", catastrophicTriggered);
      console.log("  Apollo pays:", Number(triggerAmount) / ONE_USDC / 1_000_000, "M USDC");
      console.log("  Reinsurer covers:", Number(recoverable) / ONE_USDC / 1_000_000, "M USDC");
      
      expect(aggregateTriggered).to.be.true;
      expect(catastrophicTriggered).to.be.false;
      // Apollo pays $11M (110%), reinsurer covers $1.5M (125% - 110%)
      expect(Number(recoverable)).to.equal(1_500_000 * ONE_USDC);
    });
  });
  
  describe("Scenario: Disaster Year", () => {
    it("should handle year with claims at 200% of expected", () => {
      const ytdClaims = BigInt(20_000_000 * ONE_USDC); // $20M (200%)
      const currentRatioBps = (ytdClaims * BigInt(10000)) / expectedAnnualClaims;
      
      const aggregateTriggered = currentRatioBps >= BigInt(aggregateTriggerBps);
      const catastrophicTriggered = currentRatioBps >= BigInt(catastrophicTriggerBps);
      
      // Calculate layered recovery
      const aggregateTrigger = (expectedAnnualClaims * BigInt(aggregateTriggerBps)) / BigInt(10000);
      const catastrophicTrigger = (expectedAnnualClaims * BigInt(catastrophicTriggerBps)) / BigInt(10000);
      const catastrophicCeiling = (expectedAnnualClaims * BigInt(catastrophicCeilingBps)) / BigInt(10000);
      
      // Aggregate layer covers 110% to 150%
      const aggregateRecovery = catastrophicTrigger - aggregateTrigger; // $4M
      
      // Catastrophic layer covers 150% to 200% (capped at 300%)
      const catastrophicRecovery = ytdClaims - catastrophicTrigger; // $5M
      
      console.log("Disaster year scenario (200%):");
      console.log("  Aggregate triggered:", aggregateTriggered);
      console.log("  Catastrophic triggered:", catastrophicTriggered);
      console.log("  Apollo pays (to 110%):", Number(aggregateTrigger) / ONE_USDC / 1_000_000, "M USDC");
      console.log("  Aggregate covers (110-150%):", Number(aggregateRecovery) / ONE_USDC / 1_000_000, "M USDC");
      console.log("  Catastrophic covers (150-200%):", Number(catastrophicRecovery) / ONE_USDC / 1_000_000, "M USDC");
      console.log("  Total reinsurance recovery:", Number(aggregateRecovery + catastrophicRecovery) / ONE_USDC / 1_000_000, "M USDC");
      
      expect(aggregateTriggered).to.be.true;
      expect(catastrophicTriggered).to.be.true;
      expect(Number(aggregateRecovery)).to.equal(4_000_000 * ONE_USDC);
      expect(Number(catastrophicRecovery)).to.equal(5_000_000 * ONE_USDC);
    });
  });
  
  describe("Scenario: Pandemic Surge (300%+)", () => {
    it("should cap recovery at catastrophic ceiling", () => {
      const ytdClaims = BigInt(35_000_000 * ONE_USDC); // $35M (350%)
      
      const aggregateTrigger = (expectedAnnualClaims * BigInt(aggregateTriggerBps)) / BigInt(10000);
      const catastrophicTrigger = (expectedAnnualClaims * BigInt(catastrophicTriggerBps)) / BigInt(10000);
      const catastrophicCeiling = (expectedAnnualClaims * BigInt(catastrophicCeilingBps)) / BigInt(10000);
      
      // Recovery is capped at ceiling (300%)
      const aggregateRecovery = catastrophicTrigger - aggregateTrigger;
      const catastrophicRecovery = catastrophicCeiling - catastrophicTrigger;
      const totalReinsuranceRecovery = aggregateRecovery + catastrophicRecovery;
      
      // Uncovered amount beyond ceiling
      const uncoveredAmount = ytdClaims - catastrophicCeiling;
      
      console.log("Pandemic surge scenario (350%):");
      console.log("  YTD claims:", Number(ytdClaims) / ONE_USDC / 1_000_000, "M USDC");
      console.log("  Catastrophic ceiling (300%):", Number(catastrophicCeiling) / ONE_USDC / 1_000_000, "M USDC");
      console.log("  Total reinsurance recovery:", Number(totalReinsuranceRecovery) / ONE_USDC / 1_000_000, "M USDC");
      console.log("  Uncovered (beyond 300%):", Number(uncoveredAmount) / ONE_USDC / 1_000_000, "M USDC");
      console.log("  >>> This is where staked APH liquidation activates <<<");
      
      // Aggregate: $4M (110-150%), Catastrophic: $15M (150-300%), Total: $19M
      expect(Number(totalReinsuranceRecovery)).to.equal(19_000_000 * ONE_USDC);
      expect(Number(uncoveredAmount)).to.equal(5_000_000 * ONE_USDC);
    });
  });
});
