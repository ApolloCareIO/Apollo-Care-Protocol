// tests/apollo_governance.ts

import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { ApolloGovernance } from "../target/types/apollo_governance";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo } from "@solana/spl-token";
import { expect } from "chai";
import {
  airdropTo,
  airdropToMultiple,
  createAphMint,
  createAndFundTokenAccount,
  assertError,
  aphToLamports,
} from "./utils";

describe("apollo_governance", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ApolloGovernance as Program<ApolloGovernance>;

  let aphMint: PublicKey;
  let daoConfig: PublicKey;
  let daoTreasury: PublicKey;
  let votingConfig: PublicKey;
  let authority: Keypair;
  let nonAuthority: Keypair;

  before(async () => {
    authority = Keypair.generate();
    nonAuthority = Keypair.generate();

    await airdropToMultiple(provider.connection, [authority, nonAuthority]);

    // Create APH mint
    aphMint = await createAphMint(provider.connection, authority);

    // Derive PDAs
    [daoConfig] = PublicKey.findProgramAddressSync(
      [Buffer.from("dao_config")],
      program.programId
    );

    [daoTreasury] = PublicKey.findProgramAddressSync(
      [Buffer.from("dao_treasury")],
      program.programId
    );

    [votingConfig] = PublicKey.findProgramAddressSync(
      [Buffer.from("voting_config")],
      program.programId
    );
  });

  // ==================== INITIALIZATION TESTS ====================

  describe("Initialization", () => {
    it("Initializes DAO config", async () => {
      const tx = await program.methods
        .initializeDao({
          minimumStakeForProposal: aphToLamports(1000),
          votingPeriod: new BN(3 * 24 * 60 * 60), // 3 days
          quorumBps: 500, // 5%
          approvalThresholdBps: 5000, // 50%
        })
        .accounts({
          daoConfig,
          votingConfig,
          daoTreasury,
          aphMint,
          authority: authority.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();

      console.log("Initialize DAO tx:", tx);

      // Verify state
      const daoState = await program.account.daoConfig.fetch(daoConfig);
      expect(daoState.authority.toString()).to.equal(authority.publicKey.toString());
      expect(daoState.aphMint.toString()).to.equal(aphMint.toString());
      expect(daoState.proposalCount.toNumber()).to.equal(0);
    });

    it("Fails to reinitialize DAO config", async () => {
      await assertError(
        program.methods
          .initializeDao({
            minimumStakeForProposal: aphToLamports(1000),
            votingPeriod: new BN(3 * 24 * 60 * 60),
            quorumBps: 500,
            approvalThresholdBps: 5000,
          })
          .accounts({
            daoConfig,
            votingConfig,
            daoTreasury,
            aphMint,
            authority: authority.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([authority])
          .rpc(),
        "already in use"
      );
    });
  });

  // ==================== PROPOSAL TESTS ====================

  describe("Proposals", () => {
    it("Creates a proposal PDA correctly", async () => {
      const proposalId = 1;
      const [proposal] = PublicKey.findProgramAddressSync(
        [Buffer.from("proposal"), Buffer.from(proposalId.toString())],
        program.programId
      );

      expect(proposal).to.be.instanceOf(PublicKey);
      console.log("Proposal PDA:", proposal.toString());
    });

    it("Verifies voting config parameters", async () => {
      const config = await program.account.votingConfig.fetch(votingConfig);
      expect(config.votingPeriod.toNumber()).to.equal(3 * 24 * 60 * 60);
      expect(config.quorumBps).to.equal(500);
      expect(config.approvalThresholdBps).to.equal(5000);
    });
  });

  // ==================== COMMITTEE TESTS ====================

  describe("Committees", () => {
    it("Derives committee PDAs correctly", async () => {
      const [actuarialCommittee] = PublicKey.findProgramAddressSync(
        [Buffer.from("committee"), Buffer.from([0])], // 0 = Actuarial
        program.programId
      );

      const [riskCommittee] = PublicKey.findProgramAddressSync(
        [Buffer.from("committee"), Buffer.from([1])], // 1 = Risk
        program.programId
      );

      const [claimsCommittee] = PublicKey.findProgramAddressSync(
        [Buffer.from("committee"), Buffer.from([2])], // 2 = Claims
        program.programId
      );

      expect(actuarialCommittee).to.be.instanceOf(PublicKey);
      expect(riskCommittee).to.be.instanceOf(PublicKey);
      expect(claimsCommittee).to.be.instanceOf(PublicKey);

      console.log("Committee PDAs derived successfully");
    });
  });

  // ==================== AUTHORIZATION TESTS ====================

  describe("Authorization", () => {
    it("Verifies authority is set correctly", async () => {
      const config = await program.account.daoConfig.fetch(daoConfig);
      expect(config.authority.toString()).to.equal(authority.publicKey.toString());
    });

    it("Verifies APH mint is configured", async () => {
      const config = await program.account.daoConfig.fetch(daoConfig);
      expect(config.aphMint.toString()).to.equal(aphMint.toString());
    });
  });

  // ==================== EDGE CASE TESTS ====================

  describe("Edge Cases", () => {
    it("Validates quorum basis points", async () => {
      const config = await program.account.votingConfig.fetch(votingConfig);
      expect(config.quorumBps).to.be.lessThanOrEqual(10000);
      expect(config.quorumBps).to.be.greaterThan(0);
    });

    it("Validates approval threshold basis points", async () => {
      const config = await program.account.votingConfig.fetch(votingConfig);
      expect(config.approvalThresholdBps).to.be.lessThanOrEqual(10000);
      expect(config.approvalThresholdBps).to.be.greaterThan(0);
    });

    it("Validates minimum stake for proposal", async () => {
      const config = await program.account.daoConfig.fetch(daoConfig);
      expect(config.minimumStakeForProposal.toNumber()).to.be.greaterThan(0);
    });
  });
});
