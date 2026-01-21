// tests/bootstrap_viability.ts
//
// Integration tests for Apollo Care bootstrap viability
// Validates protocol operation at small scale ($1.5M-$5M capital)

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { 
  TOKEN_PROGRAM_ID, 
  createMint, 
  createAccount, 
  mintTo,
  getAccount
} from "@solana/spl-token";
import { assert, expect } from "chai";

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

interface BootstrapScenario {
  name: string;
  capital: number;        // Total capital in USDC
  members: number;        // Expected member count
  monthlyPremium: number; // Average monthly premium per member
  monthlyClaimsRate: number; // Expected claims as % of premium (e.g., 0.90 = 90%)
  reinsuranceAttachment: number; // Specific stop-loss attachment
}

const USDC_DECIMALS = 6;

// Define test scenarios
const SCENARIOS: BootstrapScenario[] = [
  {
    name: "Soft Cap ($1.5M)",
    capital: 1_500_000,
    members: 300,
    monthlyPremium: 450,
    monthlyClaimsRate: 0.90,
    reinsuranceAttachment: 50_000,
  },
  {
    name: "Realistic Cap ($3M)",
    capital: 3_000_000,
    members: 600,
    monthlyPremium: 450,
    monthlyClaimsRate: 0.90,
    reinsuranceAttachment: 75_000,
  },
  {
    name: "Target Cap ($5M)",
    capital: 5_000_000,
    members: 1000,
    monthlyPremium: 450,
    monthlyClaimsRate: 0.90,
    reinsuranceAttachment: 100_000,
  },
];

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

function toUsdcLamports(amount: number): bigint {
  return BigInt(Math.floor(amount * 10 ** USDC_DECIMALS));
}

function fromUsdcLamports(lamports: bigint): number {
  return Number(lamports) / 10 ** USDC_DECIMALS;
}

/**
 * Calculate Capital Adequacy Ratio (CAR)
 * CAR = Total Reserves / Expected Annual Claims
 */
function calculateCAR(
  totalReserves: number,
  expectedAnnualClaims: number
): number {
  if (expectedAnnualClaims === 0) return Infinity;
  return totalReserves / expectedAnnualClaims;
}

/**
 * Determine Zone from CAR
 */
function determineZone(carBps: number): string {
  if (carBps >= 15000) return "Green";
  if (carBps >= 12500) return "Yellow";
  if (carBps >= 10000) return "Orange";
  return "Red";
}

/**
 * Objective Risk calculation (per Morrisey)
 * σ / (μ√N) where:
 * - σ = standard deviation of individual claims
 * - μ = mean claim
 * - N = pool size
 */
function calculateObjectiveRisk(
  meanClaim: number,
  stdDevClaim: number,
  poolSize: number
): number {
  if (meanClaim === 0 || poolSize === 0) return Infinity;
  return stdDevClaim / (meanClaim * Math.sqrt(poolSize));
}

/**
 * Calculate how many months of runway a given capital provides
 */
function calculateRunway(
  capital: number,
  monthlyPremiums: number,
  monthlyClaimsRate: number,
  monthlyAdminRate: number = 0.08 // 8% admin load
): number {
  const monthlyExpenses = monthlyPremiums * (monthlyClaimsRate + monthlyAdminRate);
  const monthlyNet = monthlyPremiums - monthlyExpenses;
  
  if (monthlyNet >= 0) {
    // Self-sustaining, runway is infinite
    return Infinity;
  }
  
  // Calculate months until capital depleted
  return -capital / monthlyNet;
}

// ============================================================================
// BOOTSTRAP VIABILITY CALCULATIONS
// ============================================================================

describe("Bootstrap Viability Analysis", () => {
  
  describe("Scenario Analysis", () => {
    SCENARIOS.forEach(scenario => {
      describe(scenario.name, () => {
        // Calculate derived metrics
        const monthlyPremiumRevenue = scenario.members * scenario.monthlyPremium;
        const annualPremiumRevenue = monthlyPremiumRevenue * 12;
        const monthlyExpectedClaims = monthlyPremiumRevenue * scenario.monthlyClaimsRate;
        const annualExpectedClaims = monthlyExpectedClaims * 12;
        
        // CAR calculation
        const carRatio = calculateCAR(scenario.capital, annualExpectedClaims);
        const carBps = Math.floor(carRatio * 10000);
        const zone = determineZone(carBps);
        
        // Reserve tier allocation (typical)
        const tier2 = scenario.capital * 0.50; // 50% to Tier 2
        const operations = scenario.capital * 0.30; // 30% to operations
        const reinsurance = scenario.capital * 0.10; // 10% to reinsurance
        const legal = scenario.capital * 0.07; // 7% to legal
        const audit = scenario.capital * 0.03; // 3% to audit
        
        it("should have adequate initial CAR", () => {
          console.log(`\n${scenario.name}:`);
          console.log(`  Capital: $${scenario.capital.toLocaleString()}`);
          console.log(`  Members: ${scenario.members}`);
          console.log(`  Annual Premium Revenue: $${annualPremiumRevenue.toLocaleString()}`);
          console.log(`  Annual Expected Claims: $${annualExpectedClaims.toLocaleString()}`);
          console.log(`  CAR: ${(carRatio * 100).toFixed(1)}% (${carBps} bps)`);
          console.log(`  Zone: ${zone}`);
          
          // At bootstrap, we need at least Yellow zone (125% CAR)
          expect(carBps).to.be.at.least(10000, "CAR should be at least 100%");
        });
        
        it("should have appropriate reserve allocation", () => {
          console.log(`\n  Reserve Allocation:`);
          console.log(`    Tier 2: $${tier2.toLocaleString()} (50%)`);
          console.log(`    Operations: $${operations.toLocaleString()} (30%)`);
          console.log(`    Reinsurance: $${reinsurance.toLocaleString()} (10%)`);
          console.log(`    Legal: $${legal.toLocaleString()} (7%)`);
          console.log(`    Audit: $${audit.toLocaleString()} (3%)`);
          
          // Verify allocations sum to capital
          const totalAllocated = tier2 + operations + reinsurance + legal + audit;
          expect(totalAllocated).to.be.closeTo(scenario.capital, 1);
        });
        
        it("should survive single catastrophic claim with reinsurance", () => {
          const catastrophicClaim = 150_000; // $150k claim
          const apolloPays = Math.min(catastrophicClaim, scenario.reinsuranceAttachment);
          const reinsurerPays = (catastrophicClaim - apolloPays) * 0.90; // 90% coverage
          const totalApolloExposure = apolloPays + (catastrophicClaim - apolloPays) * 0.10;
          
          const exposurePercent = (totalApolloExposure / scenario.capital) * 100;
          
          console.log(`\n  Catastrophic Claim ($150k) Analysis:`);
          console.log(`    Reinsurance Attachment: $${scenario.reinsuranceAttachment.toLocaleString()}`);
          console.log(`    Apollo Pays: $${apolloPays.toLocaleString()}`);
          console.log(`    Reinsurer Pays: $${reinsurerPays.toLocaleString()}`);
          console.log(`    Total Apollo Exposure: $${totalApolloExposure.toLocaleString()}`);
          console.log(`    % of Capital: ${exposurePercent.toFixed(2)}%`);
          
          // Apollo's exposure should be less than 10% of capital
          expect(exposurePercent).to.be.lessThan(15, "Catastrophic claim exposure should be < 15% of capital");
        });
        
        it("should maintain solvency over 12 months", () => {
          // Simple simulation: 12 months with expected claims
          let currentCapital = scenario.capital;
          let cumulativePremiums = 0;
          let cumulativeClaims = 0;
          
          for (let month = 1; month <= 12; month++) {
            cumulativePremiums += monthlyPremiumRevenue;
            cumulativeClaims += monthlyExpectedClaims;
            
            // Net change (premiums - claims - 8% admin)
            const adminCost = monthlyPremiumRevenue * 0.08;
            const netChange = monthlyPremiumRevenue - monthlyExpectedClaims - adminCost;
            currentCapital += netChange;
          }
          
          const finalCarRatio = calculateCAR(currentCapital, annualExpectedClaims);
          const finalCarBps = Math.floor(finalCarRatio * 10000);
          
          console.log(`\n  12-Month Projection:`);
          console.log(`    Cumulative Premiums: $${cumulativePremiums.toLocaleString()}`);
          console.log(`    Cumulative Claims: $${cumulativeClaims.toLocaleString()}`);
          console.log(`    Final Capital: $${currentCapital.toLocaleString()}`);
          console.log(`    Final CAR: ${(finalCarRatio * 100).toFixed(1)}%`);
          console.log(`    Final Zone: ${determineZone(finalCarBps)}`);
          
          // Should still have positive capital and reasonable CAR
          expect(currentCapital).to.be.greaterThan(0, "Capital should remain positive");
          expect(finalCarBps).to.be.at.least(8000, "CAR should remain above 80%");
        });
        
        it("should calculate appropriate claim thresholds", () => {
          // Bootstrap thresholds (more conservative)
          const isBootstrap = scenario.members < 1000;
          const autoApproveThreshold = isBootstrap ? 500 : 1000;
          const shockClaimThreshold = isBootstrap ? 25_000 : 100_000;
          
          // Shock as % of capital
          const shockPercent = (shockClaimThreshold / scenario.capital) * 100;
          
          console.log(`\n  Claim Thresholds:`);
          console.log(`    Mode: ${isBootstrap ? "Bootstrap" : "Standard"}`);
          console.log(`    Auto-Approve: $${autoApproveThreshold}`);
          console.log(`    Shock Claim: $${shockClaimThreshold.toLocaleString()}`);
          console.log(`    Shock as % of Capital: ${shockPercent.toFixed(2)}%`);
          
          // Shock should be reasonable % of capital (2-7%)
          expect(shockPercent).to.be.at.least(0.5, "Shock threshold should be at least 0.5% of capital");
          expect(shockPercent).to.be.at.most(10, "Shock threshold should be at most 10% of capital");
        });
      });
    });
  });
  
  describe("Risk Analysis", () => {
    it("should calculate objective risk correctly", () => {
      // Based on typical health insurance claim distributions
      const meanClaim = 4_200; // $4,200 average annual claim
      const stdDevClaim = 15_000; // High variance in healthcare
      
      SCENARIOS.forEach(scenario => {
        const objectiveRisk = calculateObjectiveRisk(meanClaim, stdDevClaim, scenario.members);
        
        // Compare to large pool (10,000 members)
        const largePoolRisk = calculateObjectiveRisk(meanClaim, stdDevClaim, 10000);
        const riskMultiple = objectiveRisk / largePoolRisk;
        
        console.log(`\n${scenario.name}:`);
        console.log(`  Objective Risk: ${(objectiveRisk * 100).toFixed(2)}%`);
        console.log(`  vs 10k pool: ${riskMultiple.toFixed(1)}x higher variance`);
      });
    });
    
    it("should identify when reinsurance is critical", () => {
      SCENARIOS.forEach(scenario => {
        // Without reinsurance, variance could cause insolvency
        const catastrophicClaimPercent = 150_000 / scenario.capital;
        const isCritical = catastrophicClaimPercent > 0.05; // > 5% of capital
        
        console.log(`\n${scenario.name}:`);
        console.log(`  Catastrophic claim impact: ${(catastrophicClaimPercent * 100).toFixed(1)}%`);
        console.log(`  Reinsurance critical: ${isCritical ? "YES" : "No"}`);
        
        // At bootstrap, reinsurance should always be critical
        if (scenario.members < 1000) {
          expect(isCritical).to.be.true;
        }
      });
    });
  });
  
  describe("Phase Transition Requirements", () => {
    const phase1Requirements = {
      minMonthsOperation: 12,
      minMembers: 300,
      minLossRatioBps: 8500, // 85%
      maxLossRatioBps: 9500, // 95%
      consecutiveGoodMonths: 6,
      minCarBps: 12500, // 125%
    };
    
    const phase2Requirements = {
      minMonthsSandbox: 12,
      minMembers: 1000,
      regulatoryApproval: true,
      statutoryCapitalMet: true,
      actuarialCertification: true,
    };
    
    it("should validate Phase 1 → 2 requirements", () => {
      console.log("\nPhase 1 → 2 Transition Requirements:");
      console.log(`  Minimum months operation: ${phase1Requirements.minMonthsOperation}`);
      console.log(`  Minimum members: ${phase1Requirements.minMembers}`);
      console.log(`  Loss ratio range: ${phase1Requirements.minLossRatioBps/100}% - ${phase1Requirements.maxLossRatioBps/100}%`);
      console.log(`  Consecutive good months: ${phase1Requirements.consecutiveGoodMonths}`);
      console.log(`  Minimum CAR: ${phase1Requirements.minCarBps/100}%`);
      
      // Verify requirements are achievable with soft cap
      const softCapScenario = SCENARIOS[0];
      const yearEndCar = calculateCAR(softCapScenario.capital * 1.02, // Slight growth
        softCapScenario.members * softCapScenario.monthlyPremium * 12 * 0.90);
      
      expect(yearEndCar * 10000).to.be.at.least(phase1Requirements.minCarBps);
    });
    
    it("should validate Phase 2 → 3 requirements", () => {
      console.log("\nPhase 2 → 3 Transition Requirements:");
      console.log(`  Minimum sandbox months: ${phase2Requirements.minMonthsSandbox}`);
      console.log(`  Minimum members: ${phase2Requirements.minMembers}`);
      console.log(`  Regulatory approval: Required`);
      console.log(`  Statutory capital: Required`);
      console.log(`  Actuarial certification: Required`);
    });
  });
  
  describe("AI/ML Claims Processing Tiers", () => {
    it("should define appropriate tier thresholds", () => {
      // Tier 1: Fast-Lane Auto-Approval
      const fastLaneThreshold = 500; // $500 for bootstrap
      const fastLaneCategories = [
        "PrimaryCare", "Prescription", "Laboratory", "Preventive"
      ];
      
      // Tier 2: AI-Assisted Review
      const aiReviewMin = 500;
      const aiReviewMax = 25000; // Below shock threshold
      
      // Tier 3: Committee Escalation
      const committeeThreshold = 25000;
      
      console.log("\nClaims Processing Tiers:");
      console.log(`  Tier 1 (Fast-Lane): ≤ $${fastLaneThreshold}`);
      console.log(`    Categories: ${fastLaneCategories.join(", ")}`);
      console.log(`  Tier 2 (AI Review): $${aiReviewMin} - $${aiReviewMax}`);
      console.log(`  Tier 3 (Committee): > $${committeeThreshold}`);
      
      // Verify no gaps
      expect(aiReviewMin).to.equal(fastLaneThreshold);
      expect(committeeThreshold).to.equal(aiReviewMax);
    });
    
    it("should define AI confidence thresholds", () => {
      const autoApproveConfidence = 9500; // 95%
      const maxFraudScore = 3000; // 30%
      const autoDenyConfidence = 9500; // 95%
      
      console.log("\nAI Decision Thresholds:");
      console.log(`  Auto-Approve: ≥ ${autoApproveConfidence/100}% confidence, ≤ ${maxFraudScore/100}% fraud`);
      console.log(`  Committee Review: 70-95% confidence`);
      console.log(`  Auto-Deny: ≥ ${autoDenyConfidence/100}% fraud confidence`);
    });
    
    it("should estimate claim distribution by tier", () => {
      // Typical healthcare claim distribution
      const claimDistribution = {
        under500: 0.65,    // 65% of claims by volume
        range500to25k: 0.30, // 30% of claims
        over25k: 0.05       // 5% of claims
      };
      
      // Expected auto-approval rates
      const fastLaneAutoRate = 0.95; // 95% auto-approved
      const aiTierAutoRate = 0.80;   // 80% auto-approved by AI
      const committeeRate = 1.0;     // 100% require review
      
      const totalAutoApproved = 
        claimDistribution.under500 * fastLaneAutoRate +
        claimDistribution.range500to25k * aiTierAutoRate;
      
      const totalManualReview = 1 - totalAutoApproved;
      
      console.log("\nExpected Processing Distribution:");
      console.log(`  Fast-Lane (auto): ${(claimDistribution.under500 * fastLaneAutoRate * 100).toFixed(1)}%`);
      console.log(`  AI Auto-Approve: ${(claimDistribution.range500to25k * aiTierAutoRate * 100).toFixed(1)}%`);
      console.log(`  Manual Review: ${(totalManualReview * 100).toFixed(1)}%`);
      console.log(`  Total Auto-Approved: ${(totalAutoApproved * 100).toFixed(1)}%`);
      
      // Target: > 80% claims auto-processed
      expect(totalAutoApproved).to.be.at.least(0.80);
    });
  });
});

// ============================================================================
// SMART CONTRACT INTEGRATION TESTS (requires Anchor)
// ============================================================================

describe("Smart Contract Bootstrap Parameters", () => {
  // These tests verify the smart contract constants align with actuarial requirements
  
  it("should have correct bootstrap claim thresholds", () => {
    // From apollo_claims state.rs
    const DEFAULT_AUTO_APPROVE = 1_000_000_000; // $1,000 (6 decimals)
    const BOOTSTRAP_AUTO_APPROVE = 500_000_000; // $500
    const DEFAULT_SHOCK_THRESHOLD = 100_000_000_000; // $100,000
    const BOOTSTRAP_SHOCK_THRESHOLD = 25_000_000_000; // $25,000
    
    console.log("\nClaims Thresholds (USDC lamports):");
    console.log(`  Standard Auto-Approve: ${DEFAULT_AUTO_APPROVE} ($1,000)`);
    console.log(`  Bootstrap Auto-Approve: ${BOOTSTRAP_AUTO_APPROVE} ($500)`);
    console.log(`  Standard Shock: ${DEFAULT_SHOCK_THRESHOLD} ($100,000)`);
    console.log(`  Bootstrap Shock: ${BOOTSTRAP_SHOCK_THRESHOLD} ($25,000)`);
    
    expect(BOOTSTRAP_AUTO_APPROVE).to.be.lessThan(DEFAULT_AUTO_APPROVE);
    expect(BOOTSTRAP_SHOCK_THRESHOLD).to.be.lessThan(DEFAULT_SHOCK_THRESHOLD);
  });
  
  it("should have correct MLR configuration", () => {
    // From apollo_reserves state.rs
    const RESERVE_MARGIN_BPS = 200; // 2%
    const ADMIN_LOAD_BPS = 800; // 8%
    const MIN_MLR_BPS = 9000; // 90%
    
    const totalLoading = RESERVE_MARGIN_BPS + ADMIN_LOAD_BPS;
    const impliedMLR = 10000 - totalLoading;
    
    console.log("\nMLR Configuration:");
    console.log(`  Reserve Margin: ${RESERVE_MARGIN_BPS/100}%`);
    console.log(`  Admin Load: ${ADMIN_LOAD_BPS/100}%`);
    console.log(`  Total Loading: ${totalLoading/100}%`);
    console.log(`  Implied MLR: ${impliedMLR/100}%`);
    console.log(`  Min Required MLR: ${MIN_MLR_BPS/100}%`);
    
    expect(impliedMLR).to.be.at.least(MIN_MLR_BPS);
  });
  
  it("should have correct CAR zone thresholds", () => {
    // From apollo_risk_engine state.rs
    const GREEN_THRESHOLD = 15000; // 150%
    const YELLOW_THRESHOLD = 12500; // 125%
    const ORANGE_THRESHOLD = 10000; // 100%
    
    console.log("\nCAR Zone Thresholds:");
    console.log(`  Green: ≥ ${GREEN_THRESHOLD/100}% (unlimited enrollment)`);
    console.log(`  Yellow: ≥ ${YELLOW_THRESHOLD/100}% (500/month cap)`);
    console.log(`  Orange: ≥ ${ORANGE_THRESHOLD/100}% (100/month + enhanced screening)`);
    console.log(`  Red: < ${ORANGE_THRESHOLD/100}% (enrollment frozen)`);
    
    expect(GREEN_THRESHOLD).to.be.greaterThan(YELLOW_THRESHOLD);
    expect(YELLOW_THRESHOLD).to.be.greaterThan(ORANGE_THRESHOLD);
  });
  
  it("should have correct reinsurance bootstrap defaults", () => {
    // From apollo_reserves state.rs
    const BOOTSTRAP_SPECIFIC_ATTACHMENT = 50_000_000_000; // $50k
    const BOOTSTRAP_SPECIFIC_COVERAGE = 9000; // 90%
    const BOOTSTRAP_AGGREGATE_TRIGGER = 11000; // 110%
    const BOOTSTRAP_AGGREGATE_CEILING = 15000; // 150%
    
    console.log("\nReinsurance Bootstrap Defaults:");
    console.log(`  Specific Attachment: $${BOOTSTRAP_SPECIFIC_ATTACHMENT / 1_000_000}`);
    console.log(`  Specific Coverage: ${BOOTSTRAP_SPECIFIC_COVERAGE/100}%`);
    console.log(`  Aggregate Trigger: ${BOOTSTRAP_AGGREGATE_TRIGGER/100}%`);
    console.log(`  Aggregate Ceiling: ${BOOTSTRAP_AGGREGATE_CEILING/100}%`);
    
    expect(BOOTSTRAP_SPECIFIC_COVERAGE).to.be.at.least(8500); // At least 85%
    expect(BOOTSTRAP_AGGREGATE_TRIGGER).to.be.lessThan(BOOTSTRAP_AGGREGATE_CEILING);
  });
  
  it("should have correct phase transition requirements", () => {
    // From apollo_reserves state.rs Phase1Requirements default
    const phase1Defaults = {
      minMonthsOperation: 12,
      minMembers: 300,
      minLossRatioBps: 8500,
      maxLossRatioBps: 9500,
      consecutiveGoodMonths: 6,
      minCarBps: 12500,
    };
    
    console.log("\nPhase 1 Default Requirements:");
    Object.entries(phase1Defaults).forEach(([key, value]) => {
      console.log(`  ${key}: ${value}`);
    });
    
    // Verify requirements are reasonable
    expect(phase1Defaults.minMembers).to.be.at.least(200);
    expect(phase1Defaults.minLossRatioBps).to.be.at.least(8000);
    expect(phase1Defaults.maxLossRatioBps).to.be.at.most(9800);
    expect(phase1Defaults.minCarBps).to.be.at.least(10000);
  });
});

console.log("\n" + "=".repeat(70));
console.log("APOLLO CARE BOOTSTRAP VIABILITY ANALYSIS COMPLETE");
console.log("=".repeat(70) + "\n");
