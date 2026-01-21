// tests/apollo_claims.ts
//
// Comprehensive tests for the Apollo Claims Program
// Tests: claim submission, attestation workflow, approval/denial, payment, appeals

import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { ApolloClaims } from "../target/types/apollo_claims";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getAccount } from "@solana/spl-token";
import { expect } from "chai";
import {
  airdropTo,
  airdropToMultiple,
  createUsdcMint,
  createAndFundTokenAccount,
  deriveClaimsConfig,
  deriveClaimAccount,
  deriveAttestation,
  deriveAttestorRegistry,
  deriveBenefitSchedule,
  usdcToLamports,
  lamportsToUsdc,
  assertError,
  nowSeconds,
  pastTimestamp,
} from "./utils";

describe("apollo_claims", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ApolloClaims as Program<ApolloClaims>;

  // Test accounts
  let authority: Keypair;
  let usdcMint: PublicKey;
  let claimsConfig: PublicKey;
  let attestorRegistry: PublicKey;
  let benefitSchedule: PublicKey;

  // Claim accounts
  let claimAccount1: PublicKey;
  let claimAccount2: PublicKey;

  // External program references (mocked)
  let governanceProgram: PublicKey;
  let reservesProgram: PublicKey;
  let claimsCommittee: PublicKey;

  // Members
  let member1: Keypair;
  let member1UsdcAccount: PublicKey;
  let member2: Keypair;

  // Attestors (Claims Committee members)
  let attestor1: Keypair;
  let attestor2: Keypair;
  let attestor3: Keypair;

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

    // Create USDC account for member1 (for receiving claim payments)
    member1UsdcAccount = await createAndFundTokenAccount(
      provider.connection,
      authority,
      usdcMint,
      member1.publicKey,
      0,
      authority
    );

    // Setup attestors
    attestor1 = Keypair.generate();
    attestor2 = Keypair.generate();
    attestor3 = Keypair.generate();
    await airdropToMultiple(provider.connection, [attestor1, attestor2, attestor3]);

    // Mock external programs
    governanceProgram = Keypair.generate().publicKey;
    reservesProgram = Keypair.generate().publicKey;
    claimsCommittee = Keypair.generate().publicKey;

    // Derive PDAs
    claimsConfig = deriveClaimsConfig(program.programId);
    attestorRegistry = deriveAttestorRegistry(program.programId);
    benefitSchedule = deriveBenefitSchedule(program.programId);

    claimAccount1 = deriveClaimAccount(1, program.programId);
    claimAccount2 = deriveClaimAccount(2, program.programId);
  });

  // ==================== INITIALIZATION TESTS ====================

  describe("Initialization", () => {
    it("Initializes claims configuration", async () => {
      const tx = await program.methods
        .initializeClaimsConfig({
          governanceProgram,
          reservesProgram,
          claimsCommittee,
          autoApproveThreshold: new BN(1_000 * 10 ** 6), // $1,000
          shockClaimThreshold: new BN(100_000 * 10 ** 6), // $100,000
          requiredAttestations: 2,
          maxAttestationTime: new BN(48 * 60 * 60), // 48 hours
        })
        .accounts({
          claimsConfig,
          attestorRegistry,
          authority: authority.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();

      console.log("Initialize ClaimsConfig tx:", tx);

      // Verify config
      const config = await program.account.claimsConfig.fetch(claimsConfig);
      expect(config.authority.toString()).to.equal(authority.publicKey.toString());
      expect(config.autoApproveThreshold.toNumber()).to.equal(1_000 * 10 ** 6);
      expect(config.shockClaimThreshold.toNumber()).to.equal(100_000 * 10 ** 6);
      expect(config.requiredAttestations).to.equal(2);
      expect(config.isActive).to.equal(true);
      expect(config.totalClaimsSubmitted.toNumber()).to.equal(0);
    });

    it("Sets benefit schedule", async () => {
      const tx = await program.methods
        .setBenefitSchedule({
          name: "Standard Plan",
          individualAnnualMax: new BN(500_000 * 10 ** 6), // $500k
          familyAnnualMax: new BN(1_000_000 * 10 ** 6), // $1M
          perIncidentMax: new BN(100_000 * 10 ** 6), // $100k
          individualDeductible: new BN(1_000 * 10 ** 6), // $1k
          familyDeductible: new BN(2_500 * 10 ** 6), // $2.5k
          coinsuranceBps: 8000, // 80% coverage
          oopMaxIndividual: new BN(8_000 * 10 ** 6), // $8k
          oopMaxFamily: new BN(16_000 * 10 ** 6), // $16k
          preexistingWaitingDays: 180,
          categoryLimits: [
            {
              category: { emergency: {} },
              annualLimit: new BN(100_000 * 10 ** 6),
              perVisitLimit: new BN(25_000 * 10 ** 6),
              coinsuranceOverrideBps: 9000, // 90% for emergency
            },
            {
              category: { hospitalization: {} },
              annualLimit: new BN(200_000 * 10 ** 6),
              perVisitLimit: new BN(50_000 * 10 ** 6),
              coinsuranceOverrideBps: 0, // Use default
            },
          ],
        })
        .accounts({
          claimsConfig,
          benefitSchedule,
          authority: authority.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();

      console.log("Set benefit schedule tx:", tx);

      const schedule = await program.account.benefitSchedule.fetch(benefitSchedule);
      expect(schedule.name).to.equal("Standard Plan");
      expect(schedule.individualAnnualMax.toNumber()).to.equal(500_000 * 10 ** 6);
      expect(schedule.coinsuranceBps).to.equal(8000);
      expect(schedule.isActive).to.equal(true);
    });

    it("Fails to reinitialize claims config", async () => {
      await assertError(
        program.methods
          .initializeClaimsConfig({
            governanceProgram,
            reservesProgram,
            claimsCommittee,
            autoApproveThreshold: new BN(1_000 * 10 ** 6),
            shockClaimThreshold: new BN(100_000 * 10 ** 6),
            requiredAttestations: 2,
            maxAttestationTime: new BN(48 * 60 * 60),
          })
          .accounts({
            claimsConfig,
            attestorRegistry,
            authority: authority.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([authority])
          .rpc(),
        "already in use"
      );
    });
  });

  // ==================== ATTESTOR MANAGEMENT TESTS ====================

  describe("Attestor Management", () => {
    it("Adds attestors to the registry", async () => {
      // Add attestor 1
      await program.methods
        .addAttestor(attestor1.publicKey)
        .accounts({
          claimsConfig,
          attestorRegistry,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      // Add attestor 2
      await program.methods
        .addAttestor(attestor2.publicKey)
        .accounts({
          claimsConfig,
          attestorRegistry,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      // Add attestor 3
      const tx = await program.methods
        .addAttestor(attestor3.publicKey)
        .accounts({
          claimsConfig,
          attestorRegistry,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      console.log("Add attestors tx:", tx);

      const registry = await program.account.attestorRegistry.fetch(attestorRegistry);
      expect(registry.attestorCount).to.equal(3);
      expect(registry.attestors).to.include.deep.members([
        attestor1.publicKey,
        attestor2.publicKey,
        attestor3.publicKey,
      ]);
    });

    it("Fails when non-authority tries to add attestor", async () => {
      const fakeAttestor = Keypair.generate();

      await assertError(
        program.methods
          .addAttestor(fakeAttestor.publicKey)
          .accounts({
            claimsConfig,
            attestorRegistry,
            authority: member1.publicKey, // Wrong authority
          })
          .signers([member1])
          .rpc(),
        "Unauthorized"
      );
    });

    it("Removes an attestor from the registry", async () => {
      const tx = await program.methods
        .removeAttestor(attestor3.publicKey)
        .accounts({
          claimsConfig,
          attestorRegistry,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      console.log("Remove attestor tx:", tx);

      const registry = await program.account.attestorRegistry.fetch(attestorRegistry);
      expect(registry.attestorCount).to.equal(2);
      expect(registry.attestors).to.not.include(attestor3.publicKey);
    });
  });

  // ==================== CLAIM SUBMISSION TESTS ====================

  describe("Claim Submission", () => {
    it("Submits a new claim", async () => {
      const tx = await program.methods
        .submitClaim({
          requestedAmount: new BN(5_000 * 10 ** 6), // $5,000
          category: { outpatientCare: {} },
          serviceDate: new BN(pastTimestamp(7)), // 7 days ago
          descriptionHash: "QmTestHash123456789abcdef",
          provider: null,
        })
        .accounts({
          claimsConfig,
          claimAccount: claimAccount1,
          member: member1.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([member1])
        .rpc();

      console.log("Submit claim tx:", tx);

      // Verify claim account
      const claim = await program.account.claimAccount.fetch(claimAccount1);
      expect(claim.claimId.toNumber()).to.equal(1);
      expect(claim.member.toString()).to.equal(member1.publicKey.toString());
      expect(claim.requestedAmount.toNumber()).to.equal(5_000 * 10 ** 6);
      expect(claim.status).to.deep.equal({ submitted: {} });
      expect(claim.attestationCount).to.equal(0);
      expect(claim.isShockClaim).to.equal(false);

      // Verify config updated
      const config = await program.account.claimsConfig.fetch(claimsConfig);
      expect(config.totalClaimsSubmitted.toNumber()).to.equal(1);
    });

    it("Submits a shock claim (high value)", async () => {
      const tx = await program.methods
        .submitClaim({
          requestedAmount: new BN(150_000 * 10 ** 6), // $150,000 - above shock threshold
          category: { hospitalization: {} },
          serviceDate: new BN(pastTimestamp(3)),
          descriptionHash: "QmShockClaimHash987654321",
          provider: null,
        })
        .accounts({
          claimsConfig,
          claimAccount: claimAccount2,
          member: member2.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([member2])
        .rpc();

      console.log("Submit shock claim tx:", tx);

      const claim = await program.account.claimAccount.fetch(claimAccount2);
      expect(claim.isShockClaim).to.equal(true);
      expect(claim.requestedAmount.toNumber()).to.equal(150_000 * 10 ** 6);
    });

    it("Fails to submit claim with zero amount", async () => {
      const [badClaim] = PublicKey.findProgramAddressSync(
        [Buffer.from("claim"), new BN(999).toArrayLike(Buffer, "le", 8)],
        program.programId
      );

      await assertError(
        program.methods
          .submitClaim({
            requestedAmount: new BN(0),
            category: { outpatientCare: {} },
            serviceDate: new BN(pastTimestamp(1)),
            descriptionHash: "QmZeroAmount",
            provider: null,
          })
          .accounts({
            claimsConfig,
            claimAccount: badClaim,
            member: member1.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([member1])
          .rpc(),
        "InvalidClaimAmount"
      );
    });

    it("Fails to submit claim with future service date", async () => {
      const [futureClaim] = PublicKey.findProgramAddressSync(
        [Buffer.from("claim"), new BN(998).toArrayLike(Buffer, "le", 8)],
        program.programId
      );

      await assertError(
        program.methods
          .submitClaim({
            requestedAmount: new BN(1_000 * 10 ** 6),
            category: { outpatientCare: {} },
            serviceDate: new BN(nowSeconds() + 86400), // Tomorrow
            descriptionHash: "QmFutureDate",
            provider: null,
          })
          .accounts({
            claimsConfig,
            claimAccount: futureClaim,
            member: member1.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([member1])
          .rpc(),
        "InvalidServiceDate"
      );
    });
  });

  // ==================== CLAIM STATUS TRANSITIONS ====================

  describe("Claim Status Transitions", () => {
    it("Moves claim to review status", async () => {
      const tx = await program.methods
        .moveToReview()
        .accounts({
          claimsConfig,
          claimAccount: claimAccount1,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      console.log("Move to review tx:", tx);

      const claim = await program.account.claimAccount.fetch(claimAccount1);
      expect(claim.status).to.deep.equal({ underReview: {} });
    });

    it("Moves claim to pending attestation", async () => {
      const tx = await program.methods
        .moveToPendingAttestation()
        .accounts({
          claimsConfig,
          claimAccount: claimAccount1,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      console.log("Move to pending attestation tx:", tx);

      const claim = await program.account.claimAccount.fetch(claimAccount1);
      expect(claim.status).to.deep.equal({ pendingAttestation: {} });
    });

    it("Fails to move from wrong status", async () => {
      // Claim1 is now PendingAttestation, can't move to review again
      await assertError(
        program.methods
          .moveToReview()
          .accounts({
            claimsConfig,
            claimAccount: claimAccount1,
            authority: authority.publicKey,
          })
          .signers([authority])
          .rpc(),
        "InvalidClaimStatus"
      );
    });
  });

  // ==================== ATTESTATION TESTS ====================

  describe("Attestation Workflow", () => {
    let attestation1: PublicKey;
    let attestation2: PublicKey;

    before(() => {
      attestation1 = deriveAttestation(1, attestor1.publicKey, program.programId);
      attestation2 = deriveAttestation(1, attestor2.publicKey, program.programId);
    });

    it("First attestor attests claim with approval", async () => {
      const tx = await program.methods
        .attestClaim({
          recommendation: { approveFull: {} },
          recommendedAmount: new BN(5_000 * 10 ** 6),
          notesHash: "QmAttestorNotes1",
        })
        .accounts({
          claimsConfig,
          claimAccount: claimAccount1,
          attestorRegistry,
          attestation: attestation1,
          attestor: attestor1.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([attestor1])
        .rpc();

      console.log("First attestation tx:", tx);

      // Verify attestation
      const att = await program.account.attestation.fetch(attestation1);
      expect(att.attestor.toString()).to.equal(attestor1.publicKey.toString());
      expect(att.recommendation).to.deep.equal({ approveFull: {} });
      expect(att.recommendedAmount.toNumber()).to.equal(5_000 * 10 ** 6);

      // Verify claim attestation count
      const claim = await program.account.claimAccount.fetch(claimAccount1);
      expect(claim.attestationCount).to.equal(1);
    });

    it("Second attestor attests claim", async () => {
      const tx = await program.methods
        .attestClaim({
          recommendation: { approveFull: {} },
          recommendedAmount: new BN(5_000 * 10 ** 6),
          notesHash: "QmAttestorNotes2",
        })
        .accounts({
          claimsConfig,
          claimAccount: claimAccount1,
          attestorRegistry,
          attestation: attestation2,
          attestor: attestor2.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([attestor2])
        .rpc();

      console.log("Second attestation tx:", tx);

      const claim = await program.account.claimAccount.fetch(claimAccount1);
      expect(claim.attestationCount).to.equal(2);
    });

    it("Fails when non-attestor tries to attest", async () => {
      const fakeAttestorPda = deriveAttestation(1, member1.publicKey, program.programId);

      await assertError(
        program.methods
          .attestClaim({
            recommendation: { approveFull: {} },
            recommendedAmount: new BN(5_000 * 10 ** 6),
            notesHash: "QmFakeAttestation",
          })
          .accounts({
            claimsConfig,
            claimAccount: claimAccount1,
            attestorRegistry,
            attestation: fakeAttestorPda,
            attestor: member1.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([member1])
          .rpc(),
        "AttestorNotRegistered"
      );
    });

    it("Fails when same attestor tries to attest twice", async () => {
      await assertError(
        program.methods
          .attestClaim({
            recommendation: { approveFull: {} },
            recommendedAmount: new BN(5_000 * 10 ** 6),
            notesHash: "QmDuplicateAttestation",
          })
          .accounts({
            claimsConfig,
            claimAccount: claimAccount1,
            attestorRegistry,
            attestation: attestation1,
            attestor: attestor1.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([attestor1])
          .rpc(),
        "AlreadyAttested"
      );
    });

    it("Checks attestation status", async () => {
      const status = await program.methods
        .checkAttestations()
        .accounts({
          claimsConfig,
          claimAccount: claimAccount1,
          attestorRegistry,
        })
        .view();

      console.log("Attestation status:", status);
      expect(status.hasRequiredAttestations).to.equal(true);
      expect(status.attestationCount).to.equal(2);
    });
  });

  // ==================== CLAIM RESOLUTION TESTS ====================

  describe("Claim Resolution", () => {
    it("Approves a claim", async () => {
      const approvedAmount = new BN(5_000 * 10 ** 6);

      const tx = await program.methods
        .approveClaim(approvedAmount)
        .accounts({
          claimsConfig,
          claimAccount: claimAccount1,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      console.log("Approve claim tx:", tx);

      const claim = await program.account.claimAccount.fetch(claimAccount1);
      expect(claim.status).to.deep.equal({ approved: {} });
      expect(claim.approvedAmount.toNumber()).to.equal(approvedAmount.toNumber());

      const config = await program.account.claimsConfig.fetch(claimsConfig);
      expect(config.totalClaimsApproved.toNumber()).to.equal(1);
    });

    it("Fails to approve without sufficient attestations", async () => {
      // First move claim2 to pending attestation
      await program.methods
        .moveToReview()
        .accounts({
          claimsConfig,
          claimAccount: claimAccount2,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      await program.methods
        .moveToPendingAttestation()
        .accounts({
          claimsConfig,
          claimAccount: claimAccount2,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      // Try to approve without attestations
      await assertError(
        program.methods
          .approveClaim(new BN(150_000 * 10 ** 6))
          .accounts({
            claimsConfig,
            claimAccount: claimAccount2,
            authority: authority.publicKey,
          })
          .signers([authority])
          .rpc(),
        "InsufficientAttestations"
      );
    });

    it("Denies a claim with reason", async () => {
      const [claimToDeny] = PublicKey.findProgramAddressSync(
        [Buffer.from("claim"), new BN(3).toArrayLike(Buffer, "le", 8)],
        program.programId
      );

      const member3 = Keypair.generate();
      await airdropTo(provider.connection, member3);

      // Submit a new claim
      await program.methods
        .submitClaim({
          requestedAmount: new BN(2_000 * 10 ** 6),
          category: { other: {} },
          serviceDate: new BN(pastTimestamp(1)),
          descriptionHash: "QmClaimToDeny",
          provider: null,
        })
        .accounts({
          claimsConfig,
          claimAccount: claimToDeny,
          member: member3.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([member3])
        .rpc();

      // Move through states
      await program.methods
        .moveToReview()
        .accounts({
          claimsConfig,
          claimAccount: claimToDeny,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      // Deny the claim
      const tx = await program.methods
        .denyClaim("Service not covered under benefit schedule")
        .accounts({
          claimsConfig,
          claimAccount: claimToDeny,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      console.log("Deny claim tx:", tx);

      const claim = await program.account.claimAccount.fetch(claimToDeny);
      expect(claim.status).to.deep.equal({ denied: {} });
      expect(claim.denialReason).to.equal("Service not covered under benefit schedule");

      const config = await program.account.claimsConfig.fetch(claimsConfig);
      expect(config.totalClaimsDenied.toNumber()).to.equal(1);
    });
  });

  // ==================== PAYMENT TESTS ====================

  describe("Claim Payment", () => {
    let paymentVault: PublicKey;

    before(async () => {
      // Setup payment vault (mocked reserve)
      paymentVault = await createAndFundTokenAccount(
        provider.connection,
        authority,
        usdcMint,
        authority.publicKey,
        1_000_000 * 10 ** 6, // $1M for payments
        authority
      );
    });

    it("Pays an approved claim", async () => {
      const balanceBefore = await getAccount(provider.connection, member1UsdcAccount);

      const tx = await program.methods
        .payClaim()
        .accounts({
          claimsConfig,
          claimAccount: claimAccount1,
          paymentVault,
          memberTokenAccount: member1UsdcAccount,
          authority: authority.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([authority])
        .rpc();

      console.log("Pay claim tx:", tx);

      const claim = await program.account.claimAccount.fetch(claimAccount1);
      expect(claim.status).to.deep.equal({ paid: {} });
      expect(claim.paidAmount.toNumber()).to.equal(claim.approvedAmount.toNumber());

      // Verify member received payment
      const balanceAfter = await getAccount(provider.connection, member1UsdcAccount);
      expect(Number(balanceAfter.amount) - Number(balanceBefore.amount)).to.equal(
        claim.approvedAmount.toNumber()
      );

      // Verify config totals
      const config = await program.account.claimsConfig.fetch(claimsConfig);
      expect(config.totalPaidOut.toNumber()).to.be.greaterThan(0);
    });

    it("Fails to pay claim that is not approved", async () => {
      // Create a new claim and try to pay without approval
      const [unpaidClaim] = PublicKey.findProgramAddressSync(
        [Buffer.from("claim"), new BN(4).toArrayLike(Buffer, "le", 8)],
        program.programId
      );

      const tempMember = Keypair.generate();
      await airdropTo(provider.connection, tempMember);

      const tempUsdcAccount = await createAndFundTokenAccount(
        provider.connection,
        authority,
        usdcMint,
        tempMember.publicKey,
        0,
        authority
      );

      await program.methods
        .submitClaim({
          requestedAmount: new BN(1_000 * 10 ** 6),
          category: { primaryCare: {} },
          serviceDate: new BN(pastTimestamp(2)),
          descriptionHash: "QmUnpaidClaim",
          provider: null,
        })
        .accounts({
          claimsConfig,
          claimAccount: unpaidClaim,
          member: tempMember.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([tempMember])
        .rpc();

      await assertError(
        program.methods
          .payClaim()
          .accounts({
            claimsConfig,
            claimAccount: unpaidClaim,
            paymentVault,
            memberTokenAccount: tempUsdcAccount,
            authority: authority.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([authority])
          .rpc(),
        "InvalidClaimStatus"
      );
    });

    it("Fails to pay claim twice", async () => {
      await assertError(
        program.methods
          .payClaim()
          .accounts({
            claimsConfig,
            claimAccount: claimAccount1,
            paymentVault,
            memberTokenAccount: member1UsdcAccount,
            authority: authority.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([authority])
          .rpc(),
        "AlreadyPaid"
      );
    });
  });

  // ==================== CANCELLATION TESTS ====================

  describe("Claim Cancellation", () => {
    let cancellableClaim: PublicKey;
    let cancelMember: Keypair;

    before(async () => {
      cancelMember = Keypair.generate();
      await airdropTo(provider.connection, cancelMember);

      [cancellableClaim] = PublicKey.findProgramAddressSync(
        [Buffer.from("claim"), new BN(5).toArrayLike(Buffer, "le", 8)],
        program.programId
      );

      // Submit a claim
      await program.methods
        .submitClaim({
          requestedAmount: new BN(500 * 10 ** 6),
          category: { preventive: {} },
          serviceDate: new BN(pastTimestamp(1)),
          descriptionHash: "QmCancellableClaim",
          provider: null,
        })
        .accounts({
          claimsConfig,
          claimAccount: cancellableClaim,
          member: cancelMember.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([cancelMember])
        .rpc();
    });

    it("Cancels a claim in Submitted status", async () => {
      const tx = await program.methods
        .cancelClaim()
        .accounts({
          claimsConfig,
          claimAccount: cancellableClaim,
          member: cancelMember.publicKey,
        })
        .signers([cancelMember])
        .rpc();

      console.log("Cancel claim tx:", tx);

      const claim = await program.account.claimAccount.fetch(cancellableClaim);
      expect(claim.status).to.deep.equal({ cancelled: {} });
    });

    it("Fails when non-member tries to cancel", async () => {
      // Create another claim
      const [otherClaim] = PublicKey.findProgramAddressSync(
        [Buffer.from("claim"), new BN(6).toArrayLike(Buffer, "le", 8)],
        program.programId
      );

      const realMember = Keypair.generate();
      await airdropTo(provider.connection, realMember);

      await program.methods
        .submitClaim({
          requestedAmount: new BN(300 * 10 ** 6),
          category: { laboratory: {} },
          serviceDate: new BN(pastTimestamp(1)),
          descriptionHash: "QmOtherClaim",
          provider: null,
        })
        .accounts({
          claimsConfig,
          claimAccount: otherClaim,
          member: realMember.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([realMember])
        .rpc();

      await assertError(
        program.methods
          .cancelClaim()
          .accounts({
            claimsConfig,
            claimAccount: otherClaim,
            member: member1.publicKey, // Wrong member
          })
          .signers([member1])
          .rpc(),
        "Unauthorized"
      );
    });

    it("Fails to cancel claim in later status", async () => {
      // Claim1 is now Paid - cannot cancel
      await assertError(
        program.methods
          .cancelClaim()
          .accounts({
            claimsConfig,
            claimAccount: claimAccount1,
            member: member1.publicKey,
          })
          .signers([member1])
          .rpc(),
        "CannotCancel"
      );
    });
  });

  // ==================== APPEAL TESTS ====================

  describe("Claim Appeals", () => {
    let deniedClaim: PublicKey;
    let appealMember: Keypair;

    before(async () => {
      appealMember = Keypair.generate();
      await airdropTo(provider.connection, appealMember);

      [deniedClaim] = PublicKey.findProgramAddressSync(
        [Buffer.from("claim"), new BN(7).toArrayLike(Buffer, "le", 8)],
        program.programId
      );

      // Submit and deny a claim
      await program.methods
        .submitClaim({
          requestedAmount: new BN(3_000 * 10 ** 6),
          category: { mentalHealth: {} },
          serviceDate: new BN(pastTimestamp(5)),
          descriptionHash: "QmDeniedForAppeal",
          provider: null,
        })
        .accounts({
          claimsConfig,
          claimAccount: deniedClaim,
          member: appealMember.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([appealMember])
        .rpc();

      await program.methods
        .moveToReview()
        .accounts({
          claimsConfig,
          claimAccount: deniedClaim,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      await program.methods
        .denyClaim("Documentation incomplete")
        .accounts({
          claimsConfig,
          claimAccount: deniedClaim,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();
    });

    it("Appeals a denied claim", async () => {
      const tx = await program.methods
        .appealClaim()
        .accounts({
          claimsConfig,
          claimAccount: deniedClaim,
          member: appealMember.publicKey,
        })
        .signers([appealMember])
        .rpc();

      console.log("Appeal claim tx:", tx);

      const claim = await program.account.claimAccount.fetch(deniedClaim);
      expect(claim.status).to.deep.equal({ appealed: {} });
    });

    it("Fails to appeal claim not in Denied status", async () => {
      await assertError(
        program.methods
          .appealClaim()
          .accounts({
            claimsConfig,
            claimAccount: claimAccount1, // This is Paid
            member: member1.publicKey,
          })
          .signers([member1])
          .rpc(),
        "AppealNotAllowed"
      );
    });
  });

  // ==================== CLOSE CLAIM TESTS ====================

  describe("Claim Closure", () => {
    it("Closes a paid claim", async () => {
      const tx = await program.methods
        .closeClaim()
        .accounts({
          claimsConfig,
          claimAccount: claimAccount1,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();

      console.log("Close claim tx:", tx);

      const claim = await program.account.claimAccount.fetch(claimAccount1);
      expect(claim.status).to.deep.equal({ closed: {} });
    });
  });
});
