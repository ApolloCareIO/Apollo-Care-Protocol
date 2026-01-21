// tests/apollo_membership.ts
//
// Comprehensive tests for the Apollo Membership Program
// Tests: enrollment, contributions, coverage lifecycle, persistency discounts

import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { ApolloMembership } from "../target/types/apollo_membership";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getAccount } from "@solana/spl-token";
import { expect } from "chai";
import {
  airdropTo,
  airdropToMultiple,
  createUsdcMint,
  createAndFundTokenAccount,
  deriveGlobalConfig,
  deriveMemberAccount,
  deriveEnrollmentWindow,
  deriveContributionLedger,
  usdcToLamports,
  lamportsToUsdc,
  assertError,
  nowSeconds,
  futureTimestamp,
  sleep,
} from "./utils";

describe("apollo_membership", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ApolloMembership as Program<ApolloMembership>;

  // Test accounts
  let authority: Keypair;
  let usdcMint: PublicKey;
  let globalConfig: PublicKey;
  let enrollmentWindow: PublicKey;

  // Member accounts
  let member1: Keypair;
  let member1UsdcAccount: PublicKey;
  let member1Account: PublicKey;
  let member1Ledger: PublicKey;

  let member2: Keypair;
  let member2UsdcAccount: PublicKey;

  // External program references (mocked)
  let governanceProgram: PublicKey;
  let riskEngineProgram: PublicKey;
  let reservesProgram: PublicKey;

  before(async () => {
    // Setup authority
    authority = Keypair.generate();
    await airdropTo(provider.connection, authority);

    // Create USDC mint
    usdcMint = await createUsdcMint(provider.connection, authority);

    // Setup members
    member1 = Keypair.generate();
    member2 = Keypair.generate();
    await airdropToMultiple(provider.connection, [member1, member2]);

    // Create USDC accounts for members
    member1UsdcAccount = await createAndFundTokenAccount(
      provider.connection,
      authority,
      usdcMint,
      member1.publicKey,
      10_000 * 10 ** 6, // $10,000 USDC
      authority
    );

    member2UsdcAccount = await createAndFundTokenAccount(
      provider.connection,
      authority,
      usdcMint,
      member2.publicKey,
      10_000 * 10 ** 6,
      authority
    );

    // Mock external programs
    governanceProgram = Keypair.generate().publicKey;
    riskEngineProgram = Keypair.generate().publicKey;
    reservesProgram = Keypair.generate().publicKey;

    // Derive PDAs
    globalConfig = deriveGlobalConfig(program.programId);

    const [enrollmentWindowPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("enrollment_window"), new BN(1).toArrayLike(Buffer, "le", 8)],
      program.programId
    );
    enrollmentWindow = enrollmentWindowPda;

    member1Account = deriveMemberAccount(member1.publicKey, program.programId);
    member1Ledger = deriveContributionLedger(member1.publicKey, program.programId);
  });

  // ==================== INITIALIZATION TESTS ====================

  describe("Initialization", () => {
    it("Initializes global membership config", async () => {
      const tx = await program.methods
        .initializeGlobalConfig({
          governanceProgram,
          riskEngineProgram,
          reservesProgram,
          defaultWaitingPeriodDays: 30,
          preexistingWaitingDays: 180,
          persistencyDiscountStartMonths: 12,
          persistencyDiscountBps: 500, // 5%
          maxPersistencyDiscountBps: 1000, // 10% max
        })
        .accounts({
          globalConfig,
          usdcMint,
          authority: authority.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();

      console.log("Initialize GlobalConfig tx:", tx);

      // Verify state
      const config = await program.account.globalConfig.fetch(globalConfig);
      expect(config.authority.toString()).to.equal(authority.publicKey.toString());
      expect(config.usdcMint.toString()).to.equal(usdcMint.toString());
      expect(config.totalMembers.toNumber()).to.equal(0);
      expect(config.activeMembers.toNumber()).to.equal(0);
      expect(config.defaultWaitingPeriodDays).to.equal(30);
      expect(config.persistencyDiscountBps).to.equal(500);
      expect(config.enrollmentOpen).to.equal(false);
    });

    it("Fails to reinitialize global config", async () => {
      await assertError(
        program.methods
          .initializeGlobalConfig({
            governanceProgram,
            riskEngineProgram,
            reservesProgram,
            defaultWaitingPeriodDays: 30,
            preexistingWaitingDays: 180,
            persistencyDiscountStartMonths: 12,
            persistencyDiscountBps: 500,
            maxPersistencyDiscountBps: 1000,
          })
          .accounts({
            globalConfig,
            usdcMint,
            authority: authority.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([authority])
          .rpc(),
        "already in use" // Account already initialized
      );
    });
  });

  // ==================== ENROLLMENT WINDOW TESTS ====================

  describe("Enrollment Windows", () => {
    it("Opens an enrollment window", async () => {
      const startTime = nowSeconds();
      const endTime = futureTimestamp(30); // 30 days from now

      const tx = await program.methods
        .openEnrollmentWindow({
          windowId: new BN(1),
          startTime: new BN(startTime),
          endTime: new BN(endTime),
          maxEnrollments: 1000,
          isSpecialEnrollment: false,
          description: "Q1 2024 Open Enrollment",
        })
        .accounts({
          globalConfig,
          enrollmentWindow,
          authority: authority.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();

      console.log("Open enrollment window tx:", tx);

      const window = await program.account.enrollmentWindow.fetch(enrollmentWindow);
      expect(window.windowId.toNumber()).to.equal(1);
      expect(window.isActive).to.equal(true);
      expect(window.maxEnrollments).to.equal(1000);
      expect(window.enrollmentCount).to.equal(0);
      expect(window.isSpecialEnrollment).to.equal(false);
    });

    it("Fails to open window with end time before start time", async () => {
      const [badWindow] = PublicKey.findProgramAddressSync(
        [Buffer.from("enrollment_window"), new BN(99).toArrayLike(Buffer, "le", 8)],
        program.programId
      );

      await assertError(
        program.methods
          .openEnrollmentWindow({
            windowId: new BN(99),
            startTime: new BN(futureTimestamp(30)),
            endTime: new BN(nowSeconds()), // End before start
            maxEnrollments: 100,
            isSpecialEnrollment: false,
            description: "Bad window",
          })
          .accounts({
            globalConfig,
            enrollmentWindow: badWindow,
            authority: authority.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([authority])
          .rpc(),
        "InvalidWindowConfig"
      );
    });

    it("Fails when non-authority tries to open window", async () => {
      const [badWindow] = PublicKey.findProgramAddressSync(
        [Buffer.from("enrollment_window"), new BN(98).toArrayLike(Buffer, "le", 8)],
        program.programId
      );

      await assertError(
        program.methods
          .openEnrollmentWindow({
            windowId: new BN(98),
            startTime: new BN(nowSeconds()),
            endTime: new BN(futureTimestamp(30)),
            maxEnrollments: 100,
            isSpecialEnrollment: false,
            description: "Unauthorized window",
          })
          .accounts({
            globalConfig,
            enrollmentWindow: badWindow,
            authority: member1.publicKey, // Wrong authority
            systemProgram: SystemProgram.programId,
          })
          .signers([member1])
          .rpc(),
        "Unauthorized"
      );
    });
  });

  // ==================== MEMBER ENROLLMENT TESTS ====================

  describe("Member Enrollment", () => {
    it("Enrolls a member during open enrollment", async () => {
      const tx = await program.methods
        .enrollMember({
          age: 35,
          regionCode: 0,
          isTobaccoUser: false,
          numChildren: 2,
          numAdditionalAdults: 0,
          benefitSchedule: "standard",
        })
        .accounts({
          globalConfig,
          enrollmentWindow,
          memberAccount: member1Account,
          contributionLedger: member1Ledger,
          member: member1.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([member1])
        .rpc();

      console.log("Enroll member tx:", tx);

      // Verify member account
      const memberData = await program.account.memberAccount.fetch(member1Account);
      expect(memberData.member.toString()).to.equal(member1.publicKey.toString());
      expect(memberData.age).to.equal(35);
      expect(memberData.isTobaccoUser).to.equal(false);
      expect(memberData.numChildren).to.equal(2);
      expect(memberData.status).to.deep.equal({ pendingActivation: {} });
      expect(memberData.consecutiveMonths).to.equal(0);

      // Verify global config updated
      const config = await program.account.globalConfig.fetch(globalConfig);
      expect(config.totalMembers.toNumber()).to.equal(1);

      // Verify enrollment window updated
      const window = await program.account.enrollmentWindow.fetch(enrollmentWindow);
      expect(window.enrollmentCount).to.equal(1);
    });

    it("Fails to enroll with invalid age", async () => {
      const member3 = Keypair.generate();
      await airdropTo(provider.connection, member3);
      const member3Account = deriveMemberAccount(member3.publicKey, program.programId);
      const member3Ledger = deriveContributionLedger(member3.publicKey, program.programId);

      await assertError(
        program.methods
          .enrollMember({
            age: 0, // Invalid age
            regionCode: 0,
            isTobaccoUser: false,
            numChildren: 0,
            numAdditionalAdults: 0,
            benefitSchedule: "standard",
          })
          .accounts({
            globalConfig,
            enrollmentWindow,
            memberAccount: member3Account,
            contributionLedger: member3Ledger,
            member: member3.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([member3])
          .rpc(),
        "InvalidAge"
      );
    });

    it("Fails to enroll same member twice", async () => {
      await assertError(
        program.methods
          .enrollMember({
            age: 35,
            regionCode: 0,
            isTobaccoUser: false,
            numChildren: 0,
            numAdditionalAdults: 0,
            benefitSchedule: "standard",
          })
          .accounts({
            globalConfig,
            enrollmentWindow,
            memberAccount: member1Account,
            contributionLedger: member1Ledger,
            member: member1.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([member1])
          .rpc(),
        "AlreadyEnrolled"
      );
    });
  });

  // ==================== CONTRIBUTION TESTS ====================

  describe("Contributions", () => {
    let memberVault: PublicKey;

    before(async () => {
      // Derive vault for contributions
      [memberVault] = PublicKey.findProgramAddressSync(
        [Buffer.from("member_vault"), member1.publicKey.toBuffer()],
        program.programId
      );
    });

    it("Deposits a contribution", async () => {
      const contributionAmount = 831 * 10 ** 6; // $831 USDC

      const balanceBefore = await getAccount(provider.connection, member1UsdcAccount);

      const tx = await program.methods
        .depositContribution(new BN(contributionAmount))
        .accounts({
          globalConfig,
          memberAccount: member1Account,
          contributionLedger: member1Ledger,
          memberTokenAccount: member1UsdcAccount,
          usdcMint,
          member: member1.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([member1])
        .rpc();

      console.log("Deposit contribution tx:", tx);

      // Verify ledger updated
      const ledger = await program.account.contributionLedger.fetch(member1Ledger);
      expect(ledger.totalDeposits.toNumber()).to.equal(contributionAmount);

      // Verify member account updated
      const memberData = await program.account.memberAccount.fetch(member1Account);
      expect(memberData.totalContributionsPaid.toNumber()).to.equal(contributionAmount);

      // Verify token balance decreased
      const balanceAfter = await getAccount(provider.connection, member1UsdcAccount);
      expect(Number(balanceBefore.amount) - Number(balanceAfter.amount)).to.equal(contributionAmount);
    });

    it("Fails to deposit zero amount", async () => {
      await assertError(
        program.methods
          .depositContribution(new BN(0))
          .accounts({
            globalConfig,
            memberAccount: member1Account,
            contributionLedger: member1Ledger,
            memberTokenAccount: member1UsdcAccount,
            usdcMint,
            member: member1.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([member1])
          .rpc(),
        "InsufficientContribution"
      );
    });

    it("Checks payment status", async () => {
      const status = await program.methods
        .checkPaymentStatus()
        .accounts({
          memberAccount: member1Account,
          contributionLedger: member1Ledger,
        })
        .view();

      console.log("Payment status:", status);
      expect(status).to.exist;
    });
  });

  // ==================== COVERAGE LIFECYCLE TESTS ====================

  describe("Coverage Lifecycle", () => {
    it("Fails to activate coverage before waiting period", async () => {
      await assertError(
        program.methods
          .activateCoverageIfEligible()
          .accounts({
            globalConfig,
            memberAccount: member1Account,
            contributionLedger: member1Ledger,
            member: member1.publicKey,
          })
          .signers([member1])
          .rpc(),
        "WaitingPeriodNotComplete"
      );
    });

    it("Gets member coverage status", async () => {
      const status = await program.methods
        .getMemberStatus()
        .accounts({
          memberAccount: member1Account,
        })
        .view();

      console.log("Member status:", status);
      expect(status).to.exist;
    });

    it("Suspends coverage for non-payment", async () => {
      // First we need to activate the member (in a real test this would wait)
      // For now, we test the error case for suspension

      await assertError(
        program.methods
          .suspendCoverage("Non-payment test")
          .accounts({
            globalConfig,
            memberAccount: member1Account,
            authority: authority.publicKey,
          })
          .signers([authority])
          .rpc(),
        "CannotSuspend" // Can't suspend if not active
      );
    });

    it("Fails to reinstate when not suspended", async () => {
      await assertError(
        program.methods
          .reinstateCoverage()
          .accounts({
            globalConfig,
            memberAccount: member1Account,
            contributionLedger: member1Ledger,
            member: member1.publicKey,
          })
          .signers([member1])
          .rpc(),
        "CoverageNotActive" // Wrong status
      );
    });
  });

  // ==================== QUALIFYING EVENTS TESTS ====================

  describe("Qualifying Life Events", () => {
    let specialWindow: PublicKey;
    let member3: Keypair;
    let member3Account: PublicKey;
    let member3Ledger: PublicKey;

    before(async () => {
      member3 = Keypair.generate();
      await airdropTo(provider.connection, member3);

      member3Account = deriveMemberAccount(member3.publicKey, program.programId);
      member3Ledger = deriveContributionLedger(member3.publicKey, program.programId);

      // Create special enrollment window
      [specialWindow] = PublicKey.findProgramAddressSync(
        [Buffer.from("enrollment_window"), new BN(2).toArrayLike(Buffer, "le", 8)],
        program.programId
      );

      await program.methods
        .openEnrollmentWindow({
          windowId: new BN(2),
          startTime: new BN(nowSeconds()),
          endTime: new BN(futureTimestamp(60)),
          maxEnrollments: 100,
          isSpecialEnrollment: true,
          description: "Special Enrollment Period",
        })
        .accounts({
          globalConfig,
          enrollmentWindow: specialWindow,
          authority: authority.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();
    });

    it("Enrolls member with qualifying event in special enrollment", async () => {
      // First enroll the member
      await program.methods
        .enrollMember({
          age: 28,
          regionCode: 1,
          isTobaccoUser: false,
          numChildren: 1,
          numAdditionalAdults: 1,
          benefitSchedule: "standard",
        })
        .accounts({
          globalConfig,
          enrollmentWindow: specialWindow,
          memberAccount: member3Account,
          contributionLedger: member3Ledger,
          member: member3.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([member3])
        .rpc();

      // Then set qualifying event
      const tx = await program.methods
        .setMemberQualifyingEvent({ birthAdoption: {} })
        .accounts({
          globalConfig,
          memberAccount: member3Account,
          member: member3.publicKey,
        })
        .signers([member3])
        .rpc();

      console.log("Set qualifying event tx:", tx);

      const memberData = await program.account.memberAccount.fetch(member3Account);
      expect(memberData.hasQualifyingEvent).to.equal(true);
    });
  });

  // ==================== PERSISTENCY DISCOUNT TESTS ====================

  describe("Persistency Discounts", () => {
    it("Fails to apply persistency discount before eligible", async () => {
      await assertError(
        program.methods
          .applyPersistencyDiscount()
          .accounts({
            globalConfig,
            memberAccount: member1Account,
            member: member1.publicKey,
          })
          .signers([member1])
          .rpc(),
        "PersistencyNotAvailable"
      );
    });
  });

  // ==================== ENROLLMENT WINDOW CLOSE TESTS ====================

  describe("Enrollment Window Management", () => {
    it("Closes an enrollment window", async () => {
      const tx = await program.methods
        .closeEnrollmentWindow()
        .accounts({
          globalConfig,
          enrollmentWindow,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      console.log("Close enrollment window tx:", tx);

      const window = await program.account.enrollmentWindow.fetch(enrollmentWindow);
      expect(window.isActive).to.equal(false);
    });

    it("Fails to enroll after window closes", async () => {
      const member4 = Keypair.generate();
      await airdropTo(provider.connection, member4);
      const member4Account = deriveMemberAccount(member4.publicKey, program.programId);
      const member4Ledger = deriveContributionLedger(member4.publicKey, program.programId);

      await assertError(
        program.methods
          .enrollMember({
            age: 40,
            regionCode: 0,
            isTobaccoUser: true,
            numChildren: 0,
            numAdditionalAdults: 0,
            benefitSchedule: "standard",
          })
          .accounts({
            globalConfig,
            enrollmentWindow,
            memberAccount: member4Account,
            contributionLedger: member4Ledger,
            member: member4.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([member4])
          .rpc(),
        "WindowNotActive"
      );
    });
  });

  // ==================== TERMINATION TESTS ====================

  describe("Coverage Termination", () => {
    it("Fails when non-authority tries to terminate", async () => {
      await assertError(
        program.methods
          .terminateCoverage("Unauthorized termination")
          .accounts({
            globalConfig,
            memberAccount: member1Account,
            authority: member2.publicKey, // Wrong authority
          })
          .signers([member2])
          .rpc(),
        "Unauthorized"
      );
    });
  });
});
