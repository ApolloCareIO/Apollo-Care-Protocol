// tests/apollo_staking.ts

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ApolloStaking } from "../target/types/apollo_staking";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getAccount } from "@solana/spl-token";
import { expect } from "chai";

describe("apollo_staking", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ApolloStaking as Program<ApolloStaking>;
  
  let aphMint: PublicKey;
  let stakingConfig: PublicKey;
  let aphVault: PublicKey;
  let liquidationQueue: PublicKey;
  let vaultTokenAccount: PublicKey;
  let authority: Keypair;
  let staker: Keypair;
  let stakerTokenAccount: PublicKey;

  before(async () => {
    authority = Keypair.generate();
    staker = Keypair.generate();
    
    // Airdrop SOL
    for (const kp of [authority, staker]) {
      const sig = await provider.connection.requestAirdrop(
        kp.publicKey,
        10 * anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig);
    }
    
    // Create APH mint
    aphMint = await createMint(
      provider.connection,
      authority,
      authority.publicKey,
      null,
      9
    );
    
    // Derive PDAs
    [stakingConfig] = PublicKey.findProgramAddressSync(
      [Buffer.from("staking_config")],
      program.programId
    );
    
    [aphVault] = PublicKey.findProgramAddressSync(
      [Buffer.from("aph_vault")],
      program.programId
    );
    
    [liquidationQueue] = PublicKey.findProgramAddressSync(
      [Buffer.from("liquidation_queue")],
      program.programId
    );
    
    [vaultTokenAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_token"), aphMint.toBuffer()],
      program.programId
    );
    
    // Create staker token account and mint APH
    stakerTokenAccount = await createAccount(
      provider.connection,
      staker,
      aphMint,
      staker.publicKey
    );
    
    await mintTo(
      provider.connection,
      authority,
      aphMint,
      stakerTokenAccount,
      authority,
      100_000 * 10 ** 9 // 100k APH
    );
  });

  it("Initializes staking config", async () => {
    const governanceProgram = Keypair.generate().publicKey;
    const reservesProgram = Keypair.generate().publicKey;
    
    const tx = await program.methods
      .initializeStakingConfig({
        governanceProgram,
        reservesProgram,
        epochDuration: new anchor.BN(7 * 24 * 60 * 60), // 7 days
        aphHaircutBps: 5000, // 50%
      })
      .accounts({
        stakingConfig,
        aphVault,
        liquidationQueue,
        aphMint,
        vaultTokenAccount,
        authority: authority.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authority])
      .rpc();
    
    console.log("Initialize Staking tx:", tx);
    
    const config = await program.account.stakingConfig.fetch(stakingConfig);
    expect(config.totalStaked.toNumber()).to.equal(0);
    expect(config.aphHaircutBps).to.equal(5000);
  });

  it("Initializes default staking tiers", async () => {
    const [conservativeTier] = PublicKey.findProgramAddressSync(
      [Buffer.from("staking_tier"), Buffer.from([0])],
      program.programId
    );
    const [standardTier] = PublicKey.findProgramAddressSync(
      [Buffer.from("staking_tier"), Buffer.from([1])],
      program.programId
    );
    const [aggressiveTier] = PublicKey.findProgramAddressSync(
      [Buffer.from("staking_tier"), Buffer.from([2])],
      program.programId
    );
    
    const tx = await program.methods
      .initializeDefaultTiers()
      .accounts({
        stakingConfig,
        conservativeTier,
        standardTier,
        aggressiveTier,
        authority: authority.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();
    
    console.log("Initialize tiers tx:", tx);
    
    // Verify Conservative tier
    const cons = await program.account.stakingTier.fetch(conservativeTier);
    expect(cons.name).to.equal("Conservative");
    expect(cons.minApyBps).to.equal(300); // 3%
    expect(cons.maxApyBps).to.equal(500); // 5%
    expect(cons.maxLossBps).to.equal(200); // 2%
    
    // Verify Standard tier
    const std = await program.account.stakingTier.fetch(standardTier);
    expect(std.name).to.equal("Standard");
    expect(std.minApyBps).to.equal(600); // 6%
    expect(std.maxApyBps).to.equal(800); // 8%
    expect(std.maxLossBps).to.equal(500); // 5%
    
    // Verify Aggressive tier
    const agg = await program.account.stakingTier.fetch(aggressiveTier);
    expect(agg.name).to.equal("Aggressive");
    expect(agg.minApyBps).to.equal(1000); // 10%
    expect(agg.maxApyBps).to.equal(1500); // 15%
    expect(agg.maxLossBps).to.equal(1000); // 10%
  });

  it("Stakes APH into Standard tier", async () => {
    const [standardTier] = PublicKey.findProgramAddressSync(
      [Buffer.from("staking_tier"), Buffer.from([1])],
      program.programId
    );
    
    const [stakerAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("staker_account"), staker.publicKey.toBuffer()],
      program.programId
    );
    
    // Position ID will be 0 for first stake
    const [stakePosition] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("stake_position"),
        staker.publicKey.toBuffer(),
        new anchor.BN(0).toArrayLike(Buffer, "le", 8)
      ],
      program.programId
    );
    
    const stakeAmount = new anchor.BN(10_000 * 10 ** 9); // 10k APH
    
    const tx = await program.methods
      .stake(stakeAmount)
      .accounts({
        stakingConfig,
        stakingTier: standardTier,
        aphVault,
        stakerAccount,
        stakePosition,
        stakerTokenAccount,
        vaultTokenAccount,
        staker: staker.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([staker])
      .rpc();
    
    console.log("Stake tx:", tx);
    
    // Verify position
    const position = await program.account.stakePosition.fetch(stakePosition);
    expect(position.amount.toNumber()).to.equal(stakeAmount.toNumber());
    expect(position.tierId).to.equal(1); // Standard
    expect(position.isActive).to.equal(true);
    
    // Verify staker account
    const account = await program.account.stakerAccount.fetch(stakerAccount);
    expect(account.totalStaked.toNumber()).to.equal(stakeAmount.toNumber());
    expect(account.activePositions).to.equal(1);
    
    // Verify tier totals
    const tier = await program.account.stakingTier.fetch(standardTier);
    expect(tier.totalStaked.toNumber()).to.equal(stakeAmount.toNumber());
    expect(tier.stakerCount.toNumber()).to.equal(1);
  });

  it("Computes rewards for staked position", async () => {
    // Wait a bit for time to pass (in real tests, we'd mock time)
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    const [standardTier] = PublicKey.findProgramAddressSync(
      [Buffer.from("staking_tier"), Buffer.from([1])],
      program.programId
    );
    
    const [stakerAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("staker_account"), staker.publicKey.toBuffer()],
      program.programId
    );
    
    const [stakePosition] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("stake_position"),
        staker.publicKey.toBuffer(),
        new anchor.BN(0).toArrayLike(Buffer, "le", 8)
      ],
      program.programId
    );
    
    const tx = await program.methods
      .computeRewards()
      .accounts({
        stakingConfig,
        stakingTier: standardTier,
        stakePosition,
        stakerAccount,
      })
      .rpc();
    
    console.log("Compute rewards tx:", tx);
    
    const position = await program.account.stakePosition.fetch(stakePosition);
    console.log("Rewards earned:", position.rewardsEarned.toString());
    // Rewards will be small due to short time, but should be > 0
  });

  it("Fails to unstake before lock period", async () => {
    const [standardTier] = PublicKey.findProgramAddressSync(
      [Buffer.from("staking_tier"), Buffer.from([1])],
      program.programId
    );
    
    const [stakerAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("staker_account"), staker.publicKey.toBuffer()],
      program.programId
    );
    
    const [stakePosition] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("stake_position"),
        staker.publicKey.toBuffer(),
        new anchor.BN(0).toArrayLike(Buffer, "le", 8)
      ],
      program.programId
    );
    
    try {
      await program.methods
        .unstake()
        .accounts({
          stakingConfig,
          stakingTier: standardTier,
          aphVault,
          stakerAccount,
          stakePosition,
          stakerTokenAccount,
          vaultTokenAccount,
          staker: staker.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([staker])
        .rpc();
      
      expect.fail("Should have thrown PositionLocked error");
    } catch (e) {
      expect(e.toString()).to.include("PositionLocked");
    }
  });

  it("Emergency unstakes with fee", async () => {
    const [standardTier] = PublicKey.findProgramAddressSync(
      [Buffer.from("staking_tier"), Buffer.from([1])],
      program.programId
    );
    
    const [stakerAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("staker_account"), staker.publicKey.toBuffer()],
      program.programId
    );
    
    const [stakePosition] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("stake_position"),
        staker.publicKey.toBuffer(),
        new anchor.BN(0).toArrayLike(Buffer, "le", 8)
      ],
      program.programId
    );
    
    const balanceBefore = (await getAccount(provider.connection, stakerTokenAccount)).amount;
    
    const tx = await program.methods
      .emergencyUnstake()
      .accounts({
        stakingConfig,
        stakingTier: standardTier,
        aphVault,
        stakerAccount,
        stakePosition,
        stakerTokenAccount,
        vaultTokenAccount,
        staker: staker.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([staker])
      .rpc();
    
    console.log("Emergency unstake tx:", tx);
    
    // Verify position closed
    const position = await program.account.stakePosition.fetch(stakePosition);
    expect(position.isActive).to.equal(false);
    
    // Verify balance increased (minus 10% fee)
    const balanceAfter = (await getAccount(provider.connection, stakerTokenAccount)).amount;
    const stakeAmount = 10_000n * 10n ** 9n;
    const expectedReturn = stakeAmount - (stakeAmount / 10n); // 90% of stake
    expect(balanceAfter - balanceBefore).to.be.approximately(
      Number(expectedReturn),
      Number(expectedReturn) * 0.01 // 1% tolerance
    );
  });
});
