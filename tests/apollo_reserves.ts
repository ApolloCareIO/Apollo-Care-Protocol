// tests/apollo_reserves.ts
//
// Comprehensive tests for the Apollo Reserves Program
// Tests: vault management, contribution routing, IBNR, waterfall payouts, run-off

import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { ApolloReserves } from "../target/types/apollo_reserves";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getAccount } from "@solana/spl-token";
import { expect } from "chai";
import {
  airdropTo,
  createUsdcMint,
  createAndFundTokenAccount,
  deriveReserveConfig,
  deriveReserveState,
  deriveVaultAuthority,
  usdcToLamports,
  lamportsToUsdc,
  assertError,
  nowSeconds,
} from "./utils";

describe("apollo_reserves", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ApolloReserves as Program<ApolloReserves>;

  // Test accounts
  let authority: Keypair;
  let usdcMint: PublicKey;
  let reserveConfig: PublicKey;
  let reserveState: PublicKey;
  let vaultAuthority: PublicKey;
  let runoffState: PublicKey;
  let ibnrParams: PublicKey;

  // Vault token accounts
  let tier0Vault: PublicKey;
  let tier1Vault: PublicKey;
  let tier2Vault: PublicKey;
  let runoffVault: PublicKey;
  let adminVault: PublicKey;

  // External program references (mocked)
  let governanceProgram: PublicKey;
  let riskEngineProgram: PublicKey;

  // Contributor for testing
  let contributor: Keypair;
  let contributorUsdcAccount: PublicKey;

  before(async () => {
    // Setup authority
    authority = Keypair.generate();
    await airdropTo(provider.connection, authority);

    // Create USDC mint
    usdcMint = await createUsdcMint(provider.connection, authority);

    // Setup contributor
    contributor = Keypair.generate();
    await airdropTo(provider.connection, contributor);

    contributorUsdcAccount = await createAndFundTokenAccount(
      provider.connection,
      authority,
      usdcMint,
      contributor.publicKey,
      50_000_000 * 10 ** 6, // $50M USDC (simulating ICO funds)
      authority
    );

    // Mock external programs
    governanceProgram = Keypair.generate().publicKey;
    riskEngineProgram = Keypair.generate().publicKey;

    // Derive PDAs
    reserveConfig = deriveReserveConfig(program.programId);
    reserveState = deriveReserveState(program.programId);
    vaultAuthority = deriveVaultAuthority(program.programId);

    [runoffState] = PublicKey.findProgramAddressSync(
      [Buffer.from("runoff_state")],
      program.programId
    );

    [ibnrParams] = PublicKey.findProgramAddressSync(
      [Buffer.from("ibnr_params")],
      program.programId
    );

    // Derive vault PDAs
    [tier0Vault] = PublicKey.findProgramAddressSync(
      [Buffer.from("tier0_vault")],
      program.programId
    );

    [tier1Vault] = PublicKey.findProgramAddressSync(
      [Buffer.from("tier1_vault")],
      program.programId
    );

    [tier2Vault] = PublicKey.findProgramAddressSync(
      [Buffer.from("tier2_vault")],
      program.programId
    );

    [runoffVault] = PublicKey.findProgramAddressSync(
      [Buffer.from("runoff_vault")],
      program.programId
    );

    [adminVault] = PublicKey.findProgramAddressSync(
      [Buffer.from("admin_vault")],
      program.programId
    );
  });

  // ==================== INITIALIZATION TESTS ====================

  describe("Initialization", () => {
    it("Initializes reserve configuration", async () => {
      const tx = await program.methods
        .initializeReserves({
          governanceProgram,
          riskEngineProgram,
          tier0TargetDays: 15,
          tier1TargetDays: 60,
          tier2TargetDays: 180,
          minCoverageRatioBps: 10000, // 100%
          targetCoverageRatioBps: 12500, // 125%
          reserveMarginBps: 500, // 5%
          adminLoadBps: 1000, // 10%
        })
        .accounts({
          reserveConfig,
          reserveState,
          vaultAuthority,
          runoffState,
          ibnrParams,
          usdcMint,
          authority: authority.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();

      console.log("Initialize Reserves tx:", tx);

      // Verify config
      const config = await program.account.reserveConfig.fetch(reserveConfig);
      expect(config.authority.toString()).to.equal(authority.publicKey.toString());
      expect(config.tier0TargetDays).to.equal(15);
      expect(config.tier1TargetDays).to.equal(60);
      expect(config.tier2TargetDays).to.equal(180);
      expect(config.reserveMarginBps).to.equal(500);
      expect(config.adminLoadBps).to.equal(1000);
      expect(config.isInitialized).to.equal(true);

      // Verify state
      const state = await program.account.reserveState.fetch(reserveState);
      expect(state.tier0Balance.toNumber()).to.equal(0);
      expect(state.tier1Balance.toNumber()).to.equal(0);
      expect(state.tier2Balance.toNumber()).to.equal(0);
    });

    it("Fails to reinitialize reserves", async () => {
      await assertError(
        program.methods
          .initializeReserves({
            governanceProgram,
            riskEngineProgram,
            tier0TargetDays: 15,
            tier1TargetDays: 60,
            tier2TargetDays: 180,
            minCoverageRatioBps: 10000,
            targetCoverageRatioBps: 12500,
            reserveMarginBps: 500,
            adminLoadBps: 1000,
          })
          .accounts({
            reserveConfig,
            reserveState,
            vaultAuthority,
            runoffState,
            ibnrParams,
            usdcMint,
            authority: authority.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([authority])
          .rpc(),
        "AlreadyInitialized"
      );
    });
  });

  // ==================== VAULT CREATION TESTS ====================

  describe("Vault Management", () => {
    it("Creates all USDC vaults", async () => {
      const tx = await program.methods
        .createVaults()
        .accounts({
          reserveConfig,
          vaultAuthority,
          tier0Vault,
          tier1Vault,
          tier2Vault,
          runoffVault,
          adminVault,
          usdcMint,
          authority: authority.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([authority])
        .rpc();

      console.log("Create vaults tx:", tx);

      // Verify vault authority updated
      const vAuth = await program.account.vaultAuthority.fetch(vaultAuthority);
      expect(vAuth.tier0Vault.toString()).to.equal(tier0Vault.toString());
      expect(vAuth.tier1Vault.toString()).to.equal(tier1Vault.toString());
      expect(vAuth.tier2Vault.toString()).to.equal(tier2Vault.toString());
      expect(vAuth.runoffVault.toString()).to.equal(runoffVault.toString());
    });
  });

  // ==================== RESERVE TARGETS TESTS ====================

  describe("Reserve Targets", () => {
    it("Updates reserve targets", async () => {
      const tx = await program.methods
        .setReserveTargets({
          tier0TargetDays: 20, // Increase from 15
          tier1TargetDays: 75, // Increase from 60
          tier2TargetDays: 200, // Increase from 180
        })
        .accounts({
          reserveConfig,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      console.log("Set reserve targets tx:", tx);

      const config = await program.account.reserveConfig.fetch(reserveConfig);
      expect(config.tier0TargetDays).to.equal(20);
      expect(config.tier1TargetDays).to.equal(75);
      expect(config.tier2TargetDays).to.equal(200);
    });

    it("Fails when non-authority tries to update targets", async () => {
      await assertError(
        program.methods
          .setReserveTargets({
            tier0TargetDays: 30,
            tier1TargetDays: 90,
            tier2TargetDays: 365,
          })
          .accounts({
            reserveConfig,
            authority: contributor.publicKey, // Wrong authority
          })
          .signers([contributor])
          .rpc(),
        "Unauthorized"
      );
    });

    it("Fails with invalid target days", async () => {
      await assertError(
        program.methods
          .setReserveTargets({
            tier0TargetDays: 0, // Invalid
            tier1TargetDays: 60,
            tier2TargetDays: 180,
          })
          .accounts({
            reserveConfig,
            authority: authority.publicKey,
          })
          .signers([authority])
          .rpc(),
        "InvalidTargetDays"
      );
    });
  });

  // ==================== CONTRIBUTION ROUTING TESTS ====================

  describe("Contribution Routing", () => {
    it("Routes contribution to vaults", async () => {
      const contributionAmount = new BN(1_000_000 * 10 ** 6); // $1M USDC

      const tx = await program.methods
        .routeContributionToVaults(contributionAmount)
        .accounts({
          reserveConfig,
          reserveState,
          vaultAuthority,
          tier0Vault,
          tier1Vault,
          tier2Vault,
          adminVault,
          sourceTokenAccount: contributorUsdcAccount,
          depositor: contributor.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([contributor])
        .rpc();

      console.log("Route contribution tx:", tx);

      // Verify reserve state updated
      const state = await program.account.reserveState.fetch(reserveState);
      expect(state.totalContributionsReceived.toNumber()).to.be.greaterThan(0);

      // Verify some funds in each tier
      // Distribution depends on implementation (typically fills Tier 0 first)
    });

    it("Fails to route zero amount", async () => {
      await assertError(
        program.methods
          .routeContributionToVaults(new BN(0))
          .accounts({
            reserveConfig,
            reserveState,
            vaultAuthority,
            tier0Vault,
            tier1Vault,
            tier2Vault,
            adminVault,
            sourceTokenAccount: contributorUsdcAccount,
            depositor: contributor.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([contributor])
          .rpc(),
        "ZeroAmount"
      );
    });

    it("Direct deposits to specific tier", async () => {
      const depositAmount = new BN(5_000_000 * 10 ** 6); // $5M USDC

      const tx = await program.methods
        .depositToTier({ tier2: {} }, depositAmount)
        .accounts({
          reserveConfig,
          reserveState,
          vaultAuthority,
          targetVault: tier2Vault,
          sourceTokenAccount: contributorUsdcAccount,
          depositor: contributor.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([contributor])
        .rpc();

      console.log("Direct deposit to Tier 2 tx:", tx);

      const state = await program.account.reserveState.fetch(reserveState);
      expect(state.tier2Balance.toNumber()).to.be.greaterThan(0);
    });
  });

  // ==================== IBNR TESTS ====================

  describe("IBNR Management", () => {
    it("Updates expected claims data", async () => {
      const tx = await program.methods
        .updateExpectedClaims({
          avgDailyClaims30d: new BN(100_000 * 10 ** 6), // $100k/day
          avgDailyClaims90d: new BN(95_000 * 10 ** 6), // $95k/day
          claimsStdDev: new BN(20_000 * 10 ** 6), // $20k std dev
          sampleSize: 90,
        })
        .accounts({
          reserveConfig,
          reserveState,
          ibnrParams,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      console.log("Update expected claims tx:", tx);

      const params = await program.account.ibnrParams.fetch(ibnrParams);
      expect(params.avgDailyClaims30d.toNumber()).to.equal(100_000 * 10 ** 6);
      expect(params.sampleSize).to.equal(90);
    });

    it("Updates IBNR parameters", async () => {
      const tx = await program.methods
        .updateIbnrParams(
          21, // 21 day reporting lag per actuarial spec
          11500 // 1.15 development factor
        )
        .accounts({
          reserveConfig,
          reserveState,
          ibnrParams,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      console.log("Update IBNR params tx:", tx);

      const state = await program.account.reserveState.fetch(reserveState);
      expect(state.avgReportingLagDays).to.equal(21);
      expect(state.developmentFactorBps).to.equal(11500);
    });

    it("Computes IBNR reserve", async () => {
      const tx = await program.methods
        .computeIbnr()
        .accounts({
          reserveConfig,
          reserveState,
          ibnrParams,
        })
        .rpc();

      console.log("Compute IBNR tx:", tx);

      const state = await program.account.reserveState.fetch(reserveState);
      // IBNR = daily_claims * reporting_lag * dev_factor
      // = $100k * 14 * 1.15 = ~$1.61M
      expect(state.ibnrUsdc.toNumber()).to.be.greaterThan(0);
      console.log("Computed IBNR:", lamportsToUsdc(state.ibnrUsdc), "USDC");
    });

    it("Fails to update IBNR with invalid development factor", async () => {
      await assertError(
        program.methods
          .updateIbnrParams(
            14,
            5000 // Less than 1.0, invalid
          )
          .accounts({
            reserveConfig,
            reserveState,
            ibnrParams,
            authority: authority.publicKey,
          })
          .signers([authority])
          .rpc(),
        "InvalidDevFactor"
      );
    });
  });

  // ==================== RUN-OFF RESERVE TESTS ====================

  describe("Run-off Reserve", () => {
    it("Sets run-off parameters", async () => {
      const tx = await program.methods
        .setRunoffParams({
          estimatedLegalCosts: new BN(500_000 * 10 ** 6), // $500k legal
          monthlyAdminCosts: new BN(100_000 * 10 ** 6), // $100k/month admin
          winddownMonths: 6,
        })
        .accounts({
          reserveConfig,
          runoffState,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      console.log("Set run-off params tx:", tx);

      const runoff = await program.account.runoffState.fetch(runoffState);
      expect(runoff.estimatedLegalCosts.toNumber()).to.equal(500_000 * 10 ** 6);
      expect(runoff.monthlyAdminCosts.toNumber()).to.equal(100_000 * 10 ** 6);
      expect(runoff.winddownMonths).to.equal(6);
    });

    it("Funds run-off reserve", async () => {
      const fundAmount = new BN(2_000_000 * 10 ** 6); // $2M

      const tx = await program.methods
        .fundRunoffReserve(fundAmount)
        .accounts({
          reserveConfig,
          reserveState,
          runoffState,
          vaultAuthority,
          runoffVault,
          sourceTokenAccount: contributorUsdcAccount,
          depositor: contributor.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([contributor])
        .rpc();

      console.log("Fund run-off reserve tx:", tx);

      const state = await program.account.reserveState.fetch(reserveState);
      expect(state.runoffBalance.toNumber()).to.equal(fundAmount.toNumber());
    });

    it("Fails to activate run-off without authority", async () => {
      await assertError(
        program.methods
          .activateRunoff()
          .accounts({
            reserveConfig,
            runoffState,
            authority: contributor.publicKey, // Wrong authority
          })
          .signers([contributor])
          .rpc(),
        "Unauthorized"
      );
    });

    it("Fails to access run-off in normal operations", async () => {
      await assertError(
        program.methods
          .emergencySpendRunoff(
            new BN(100_000 * 10 ** 6),
            "Test emergency spend"
          )
          .accounts({
            reserveConfig,
            reserveState,
            runoffState,
            vaultAuthority,
            runoffVault,
            destinationTokenAccount: contributorUsdcAccount,
            authority: authority.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([authority])
          .rpc(),
        "RunoffNotActive"
      );
    });
  });

  // ==================== TIER REFILL TESTS ====================

  describe("Tier Refills", () => {
    it("Refills Tier 0 from Tier 1", async () => {
      // First ensure Tier 1 has funds
      const depositAmount = new BN(1_000_000 * 10 ** 6);
      await program.methods
        .depositToTier({ tier1: {} }, depositAmount)
        .accounts({
          reserveConfig,
          reserveState,
          vaultAuthority,
          targetVault: tier1Vault,
          sourceTokenAccount: contributorUsdcAccount,
          depositor: contributor.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([contributor])
        .rpc();

      const stateBefore = await program.account.reserveState.fetch(reserveState);
      const tier1Before = stateBefore.tier1Balance.toNumber();

      const refillAmount = new BN(100_000 * 10 ** 6); // $100k
      const tx = await program.methods
        .refillTier0(refillAmount)
        .accounts({
          reserveConfig,
          reserveState,
          vaultAuthority,
          tier0Vault,
          tier1Vault,
          authority: authority.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([authority])
        .rpc();

      console.log("Refill Tier 0 tx:", tx);

      const stateAfter = await program.account.reserveState.fetch(reserveState);
      expect(stateAfter.tier0Balance.toNumber()).to.be.greaterThan(stateBefore.tier0Balance.toNumber());
    });

    it("Fails to refill Tier 0 with insufficient Tier 1 balance", async () => {
      // Try to refill more than available in Tier 1
      const state = await program.account.reserveState.fetch(reserveState);
      const excessiveAmount = new BN(state.tier1Balance.toNumber() + 1_000_000_000);

      await assertError(
        program.methods
          .refillTier0(excessiveAmount)
          .accounts({
            reserveConfig,
            reserveState,
            vaultAuthority,
            tier0Vault,
            tier1Vault,
            authority: authority.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([authority])
          .rpc(),
        "InsufficientTier1"
      );
    });
  });

  // ==================== WATERFALL PAYOUT TESTS ====================

  describe("Waterfall Payouts", () => {
    let claimRecipient: Keypair;
    let recipientUsdcAccount: PublicKey;

    before(async () => {
      claimRecipient = Keypair.generate();
      await airdropTo(provider.connection, claimRecipient);

      recipientUsdcAccount = await createAndFundTokenAccount(
        provider.connection,
        authority,
        usdcMint,
        claimRecipient.publicKey,
        0, // Start with 0
        authority
      );
    });

    it("Pays claim using waterfall mechanism", async () => {
      const payoutAmount = new BN(50_000 * 10 ** 6); // $50k claim

      const tx = await program.methods
        .payoutClaimFromWaterfall({
          claimId: new BN(1),
          amount: payoutAmount,
          member: claimRecipient.publicKey,
        })
        .accounts({
          reserveConfig,
          reserveState,
          vaultAuthority,
          tier0Vault,
          tier1Vault,
          tier2Vault,
          destinationTokenAccount: recipientUsdcAccount,
          authority: authority.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([authority])
        .rpc();

      console.log("Waterfall payout tx:", tx);

      // Verify recipient received funds
      const recipientBalance = await getAccount(provider.connection, recipientUsdcAccount);
      expect(Number(recipientBalance.amount)).to.equal(payoutAmount.toNumber());

      // Verify claims paid counter increased
      const state = await program.account.reserveState.fetch(reserveState);
      expect(state.totalClaimsPaid.toNumber()).to.be.greaterThan(0);
    });

    it("Fails payout with zero amount", async () => {
      await assertError(
        program.methods
          .payoutClaimFromWaterfall({
            claimId: new BN(2),
            amount: new BN(0),
            member: claimRecipient.publicKey,
          })
          .accounts({
            reserveConfig,
            reserveState,
            vaultAuthority,
            tier0Vault,
            tier1Vault,
            tier2Vault,
            destinationTokenAccount: recipientUsdcAccount,
            authority: authority.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([authority])
          .rpc(),
        "InvalidPayoutAmount"
      );
    });
  });

  // ==================== SNAPSHOT TESTS ====================

  describe("Reserve Snapshots", () => {
    it("Takes a reserve state snapshot", async () => {
      const tx = await program.methods
        .takeReserveSnapshot()
        .accounts({
          reserveConfig,
          reserveState,
          vaultAuthority,
          tier0Vault,
          tier1Vault,
          tier2Vault,
          runoffVault,
        })
        .rpc();

      console.log("Take snapshot tx:", tx);

      // Snapshot emits an event - in real tests we'd verify the event
      const state = await program.account.reserveState.fetch(reserveState);
      expect(state.lastWaterfallAt.toNumber()).to.be.greaterThan(0);
    });
  });

  // ==================== COVERAGE RATIO TESTS ====================

  describe("Coverage Ratio", () => {
    it("Verifies coverage ratio calculation", async () => {
      const state = await program.account.reserveState.fetch(reserveState);

      // Total reserves
      const totalReserves =
        state.tier0Balance.toNumber() +
        state.tier1Balance.toNumber() +
        state.tier2Balance.toNumber();

      console.log("Reserve Status:");
      console.log("  Tier 0:", lamportsToUsdc(state.tier0Balance), "USDC");
      console.log("  Tier 1:", lamportsToUsdc(state.tier1Balance), "USDC");
      console.log("  Tier 2:", lamportsToUsdc(state.tier2Balance), "USDC");
      console.log("  Total:", lamportsToUsdc(new BN(totalReserves)), "USDC");
      console.log("  IBNR:", lamportsToUsdc(state.ibnrUsdc), "USDC");
      console.log("  Coverage Ratio:", state.currentCoverageRatioBps / 100, "%");

      expect(state.currentCoverageRatioBps).to.be.greaterThan(0);
    });
  });
});
