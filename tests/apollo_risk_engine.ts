// tests/apollo_risk_engine.ts

import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { ApolloRiskEngine } from "../target/types/apollo_risk_engine";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";
import {
  airdropTo,
  airdropToMultiple,
  assertError,
  usdcToLamports,
} from "./utils";

describe("apollo_risk_engine", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ApolloRiskEngine as Program<ApolloRiskEngine>;

  let riskConfig: PublicKey;
  let ratingTable: PublicKey;
  let carState: PublicKey;
  let zoneState: PublicKey;
  let authority: Keypair;
  let nonAuthority: Keypair;

  before(async () => {
    authority = Keypair.generate();
    nonAuthority = Keypair.generate();

    await airdropToMultiple(provider.connection, [authority, nonAuthority]);

    // Derive PDAs
    [riskConfig] = PublicKey.findProgramAddressSync(
      [Buffer.from("risk_config")],
      program.programId
    );

    [ratingTable] = PublicKey.findProgramAddressSync(
      [Buffer.from("rating_table")],
      program.programId
    );

    [carState] = PublicKey.findProgramAddressSync(
      [Buffer.from("car_state")],
      program.programId
    );

    [zoneState] = PublicKey.findProgramAddressSync(
      [Buffer.from("zone_state")],
      program.programId
    );
  });

  // ==================== INITIALIZATION TESTS ====================

  describe("Initialization", () => {
    it("Initializes risk config with CMS-compliant age bands", async () => {
      const governanceProgram = Keypair.generate().publicKey;
      const reservesProgram = Keypair.generate().publicKey;

      const tx = await program.methods
        .initializeRiskConfig({
          governanceProgram,
          reservesProgram,
          baseRateAdult: usdcToLamports(450), // $450 USDC
          initialExpectedAnnualClaims: usdcToLamports(10_000_000), // $10M
        })
        .accounts({
          riskConfig,
          ratingTable,
          carState,
          zoneState,
          authority: authority.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();

      console.log("Initialize Risk Config tx:", tx);

      // Verify state
      const config = await program.account.riskConfig.fetch(riskConfig);
      expect(config.baseRateAdult.toNumber()).to.equal(450_000_000);
      expect(config.shockFactorBps).to.equal(10000); // 1.0x default
      expect(config.tobaccoFactorBps).to.equal(12000); // 1.2x

      // Verify rating table has CMS-compliant bands
      const table = await program.account.ratingTable.fetch(ratingTable);
      expect(table.bandCount).to.equal(10); // 10 default age bands

      // Verify 3:1 ratio compliance
      const factors = table.ageBands.map((b: any) => b.factorBps);
      const minFactor = Math.min(...factors);
      const maxFactor = Math.max(...factors);
      expect(maxFactor / minFactor).to.be.lessThanOrEqual(3.0);
    });

    it("Fails to reinitialize risk config", async () => {
      const governanceProgram = Keypair.generate().publicKey;
      const reservesProgram = Keypair.generate().publicKey;

      await assertError(
        program.methods
          .initializeRiskConfig({
            governanceProgram,
            reservesProgram,
            baseRateAdult: usdcToLamports(450),
            initialExpectedAnnualClaims: usdcToLamports(10_000_000),
          })
          .accounts({
            riskConfig,
            ratingTable,
            carState,
            zoneState,
            authority: authority.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([authority])
          .rpc(),
        "already in use"
      );
    });
  });

  // ==================== CONTRIBUTION QUOTE TESTS ====================

  describe("Contribution Quotes", () => {
    it("Quotes contribution for a 35-year-old with 2 children", async () => {
      const quote = await program.methods
        .quoteContribution({
          age: 35,
          isTobaccoUser: false,
          regionCode: 0,
          numChildren: 2,
          numAdditionalAdults: 0,
          additionalAdultAges: [],
        })
        .accounts({
          riskConfig,
          ratingTable,
        })
        .view();

      console.log("Contribution quote:", quote);

      // Base: $450 * 1.046 (age factor for 35-39) = ~$471
      // + 2 children at 40% = $180 * 2 = $360
      // Total ~$831 before region/shock
      expect(quote.finalContribution.toNumber()).to.be.greaterThan(0);
    });

    it("Quotes higher contribution for tobacco user", async () => {
      const nonTobaccoQuote = await program.methods
        .quoteContribution({
          age: 40,
          isTobaccoUser: false,
          regionCode: 0,
          numChildren: 0,
          numAdditionalAdults: 0,
          additionalAdultAges: [],
        })
        .accounts({
          riskConfig,
          ratingTable,
        })
        .view();

      const tobaccoQuote = await program.methods
        .quoteContribution({
          age: 40,
          isTobaccoUser: true,
          regionCode: 0,
          numChildren: 0,
          numAdditionalAdults: 0,
          additionalAdultAges: [],
        })
        .accounts({
          riskConfig,
          ratingTable,
        })
        .view();

      // Tobacco factor is 1.2x
      expect(tobaccoQuote.finalContribution.toNumber()).to.be.greaterThan(
        nonTobaccoQuote.finalContribution.toNumber()
      );
    });

    it("Quotes correctly for elderly member (highest age band)", async () => {
      const quote = await program.methods
        .quoteContribution({
          age: 64,
          isTobaccoUser: false,
          regionCode: 0,
          numChildren: 0,
          numAdditionalAdults: 0,
          additionalAdultAges: [],
        })
        .accounts({
          riskConfig,
          ratingTable,
        })
        .view();

      // Older members pay more (up to 3x base)
      expect(quote.finalContribution.toNumber()).to.be.greaterThan(450_000_000);
    });

    it("Quotes correctly for young member (lowest age band)", async () => {
      const quote = await program.methods
        .quoteContribution({
          age: 21,
          isTobaccoUser: false,
          regionCode: 0,
          numChildren: 0,
          numAdditionalAdults: 0,
          additionalAdultAges: [],
        })
        .accounts({
          riskConfig,
          ratingTable,
        })
        .view();

      // Young members pay less
      expect(quote.finalContribution.toNumber()).to.be.lessThan(900_000_000);
    });
  });

  // ==================== CAR STATE TESTS ====================

  describe("CAR State", () => {
    it("Updates CAR state and determines zone", async () => {
      const tx = await program.methods
        .updateCarState({
          totalUsdcReserves: usdcToLamports(15_000_000), // $15M
          eligibleAphUsdc: usdcToLamports(5_000_000), // $5M eligible APH
          expectedAnnualClaims: usdcToLamports(10_000_000), // $10M
        })
        .accounts({
          riskConfig,
          carState,
          zoneState,
          updater: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      console.log("Update CAR tx:", tx);

      // CAR = ($15M + $5M) / $10M = 200% -> Green zone
      const state = await program.account.carState.fetch(carState);
      expect(state.currentCarBps).to.be.greaterThan(15000); // > 150%

      const zone = await program.account.zoneState.fetch(zoneState);
      expect(zone.enrollmentFrozen).to.equal(false);
    });

    it("Triggers Yellow zone on lower CAR", async () => {
      // Reduce reserves to trigger Yellow zone (125-150%)
      await program.methods
        .updateCarState({
          totalUsdcReserves: usdcToLamports(13_000_000), // $13M
          eligibleAphUsdc: usdcToLamports(500_000), // $0.5M
          expectedAnnualClaims: usdcToLamports(10_000_000), // $10M
        })
        .accounts({
          riskConfig,
          carState,
          zoneState,
          updater: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      const car = await program.account.carState.fetch(carState);
      // CAR = 135%
      expect(car.currentCarBps).to.be.lessThan(15000);
      expect(car.currentCarBps).to.be.greaterThan(12500);
    });

    it("Triggers Orange zone on lower CAR", async () => {
      await program.methods
        .updateCarState({
          totalUsdcReserves: usdcToLamports(11_000_000), // $11M
          eligibleAphUsdc: usdcToLamports(500_000), // $0.5M
          expectedAnnualClaims: usdcToLamports(10_000_000), // $10M
        })
        .accounts({
          riskConfig,
          carState,
          zoneState,
          updater: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      const car = await program.account.carState.fetch(carState);
      // CAR = 115%
      expect(car.currentCarBps).to.be.lessThan(12500);
      expect(car.currentCarBps).to.be.greaterThan(10000);
    });
  });

  // ==================== SHOCK FACTOR TESTS ====================

  describe("ShockFactor", () => {
    before(async () => {
      // Reset to Green zone for shock factor tests
      await program.methods
        .updateCarState({
          totalUsdcReserves: usdcToLamports(20_000_000), // $20M
          eligibleAphUsdc: usdcToLamports(5_000_000), // $5M
          expectedAnnualClaims: usdcToLamports(10_000_000), // $10M
        })
        .accounts({
          riskConfig,
          carState,
          zoneState,
          updater: authority.publicKey,
        })
        .signers([authority])
        .rpc();
    });

    it("Sets ShockFactor within zone limits", async () => {
      // In Green zone, can set up to 1.25x auto
      const tx = await program.methods
        .setShockFactor(11000) // 1.1x
        .accounts({
          riskConfig,
          carState,
          zoneState,
          setter: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      console.log("Set ShockFactor tx:", tx);

      const config = await program.account.riskConfig.fetch(riskConfig);
      expect(config.shockFactorBps).to.equal(11000);
    });

    it("Fails to set ShockFactor below minimum", async () => {
      await assertError(
        program.methods
          .setShockFactor(5000) // 0.5x - below 1.0x minimum
          .accounts({
            riskConfig,
            carState,
            zoneState,
            setter: authority.publicKey,
          })
          .signers([authority])
          .rpc(),
        "InvalidShockFactor"
      );
    });

    it("Fails to set ShockFactor above zone limit", async () => {
      await assertError(
        program.methods
          .setShockFactor(20000) // 2.0x - above max
          .accounts({
            riskConfig,
            carState,
            zoneState,
            setter: authority.publicKey,
          })
          .signers([authority])
          .rpc(),
        "ShockFactorExceedsZoneLimit"
      );
    });
  });

  // ==================== ENROLLMENT TESTS ====================

  describe("Enrollment", () => {
    it("Records enrollment and checks caps", async () => {
      const member = Keypair.generate();

      const tx = await program.methods
        .recordEnrollment()
        .accounts({
          zoneState,
          member: member.publicKey,
        })
        .rpc();

      console.log("Record enrollment tx:", tx);

      const zone = await program.account.zoneState.fetch(zoneState);
      expect(zone.currentMonthEnrollments).to.be.greaterThan(0);
    });
  });

  // ==================== CMS COMPLIANCE TESTS ====================

  describe("CMS Compliance", () => {
    it("Verifies 3:1 age ratio across all bands", async () => {
      const table = await program.account.ratingTable.fetch(ratingTable);

      const factors = table.ageBands.map((b: any) => b.factorBps);
      const minFactor = Math.min(...factors);
      const maxFactor = Math.max(...factors);

      console.log(`Age band factor range: ${minFactor / 10000}x - ${maxFactor / 10000}x`);
      console.log(`Ratio: ${(maxFactor / minFactor).toFixed(2)}:1`);

      expect(maxFactor / minFactor).to.be.lessThanOrEqual(3.0);
    });

    it("Verifies tobacco factor is within ACA limits", async () => {
      const config = await program.account.riskConfig.fetch(riskConfig);

      // ACA allows up to 1.5x for tobacco
      expect(config.tobaccoFactorBps).to.be.lessThanOrEqual(15000);
      expect(config.tobaccoFactorBps).to.be.greaterThanOrEqual(10000);
    });

    it("Verifies child pricing at correct factor", async () => {
      // Children should be priced at ~40% of adult rate
      const adultQuote = await program.methods
        .quoteContribution({
          age: 35,
          isTobaccoUser: false,
          regionCode: 0,
          numChildren: 0,
          numAdditionalAdults: 0,
          additionalAdultAges: [],
        })
        .accounts({
          riskConfig,
          ratingTable,
        })
        .view();

      const withChildQuote = await program.methods
        .quoteContribution({
          age: 35,
          isTobaccoUser: false,
          regionCode: 0,
          numChildren: 1,
          numAdditionalAdults: 0,
          additionalAdultAges: [],
        })
        .accounts({
          riskConfig,
          ratingTable,
        })
        .view();

      const childCost = withChildQuote.finalContribution.toNumber() - adultQuote.finalContribution.toNumber();
      const childFactor = childCost / adultQuote.finalContribution.toNumber();

      // Child should be ~30-50% of adult cost
      expect(childFactor).to.be.greaterThan(0.25);
      expect(childFactor).to.be.lessThan(0.55);
    });
  });
});
