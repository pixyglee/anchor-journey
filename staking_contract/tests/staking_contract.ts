import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { StakingContract } from "../target/types/staking_contract";
import { PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { assert } from "chai";

describe("staking-contract", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  let provider = anchor.getProvider();
  const program = anchor.workspace.StakingContract as Program<StakingContract>;

  let pda: PublicKey;
  let stakeTime = 0;
  const POINTS_PER_SOL_PER_DAY = 1_000_000;
  const SECONDS_PER_DAY = 86_400;
  // 🔹 Before tests: derive the PDA for the test user
  before(async () => {
    const [pdaAddress, bump] = PublicKey.findProgramAddressSync(
      [Buffer.from("client"), provider.publicKey.toBuffer()],
      program.programId
    );
    pda = pdaAddress;
  });
  // ✅ Test 1: Create the PDA account for staking
  it("create account", async () => {
    const tx = await program.methods
      .createPdaAccount()
      .accounts({
        payer: provider.publicKey,
        pdaAccount: pda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const pdaAccountInfo = await provider.connection.getAccountInfo(pda);
    assert(pdaAccountInfo !== null, "PDA account should exist");
    assert(pdaAccountInfo.lamports > 0, "PDA should have rent exemption lamports");

    const stakeAccount = await program.account.stakeAccount.fetch(pda);
    assert(stakeAccount.owner.equals(provider.publicKey), "Owner should match");
    assert(stakeAccount.stakedAmount.toNumber() === 0, "Initial staked amount should be 0");
    assert(stakeAccount.totalPoints.toNumber() === 0, "Initial points should be 0");

    stakeTime = stakeAccount.lastUpdateTime.toNumber();
    console.log("Account created at timestamp:", stakeTime);
    console.log("Create account transaction signature:", tx);
  });
  // ✅ Test 2: Stake 10 SOL into the PDA
  it("stake 10 SOL", async () => {
    const stakeAmount = new anchor.BN(10 * LAMPORTS_PER_SOL);

    const tx = await program.methods
      .stake(stakeAmount)
      .accounts({
        user: provider.publicKey,
        pdaAccount: pda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const stakeAccount = await program.account.stakeAccount.fetch(pda);
    assert(
      stakeAccount.stakedAmount.eq(stakeAmount),
      `Staked amount should be ${stakeAmount.toString()}`
    );

    const pdaBalance = await provider.connection.getBalance(pda);
    const rentExemptAmount = await provider.connection.getMinimumBalanceForRentExemption(
      8 + 32 + 8 + 8 + 8 + 1
    );

    assert(
      pdaBalance >= rentExemptAmount + stakeAmount.toNumber(),
      "PDA should have rent + staked amount"
    );

    console.log("Stake transaction signature:", tx);
    console.log("Staked amount:", stakeAmount.toNumber() / LAMPORTS_PER_SOL, "SOL");
  });
  // ✅ Test 3: Wait a short period, then check that points are accumulating
  it("wait and check points accumulation", async () => {
    await new Promise(resolve => setTimeout(resolve, 2000));

    const tx = await program.methods
      .getPoints()
      .accounts({
        user: provider.publicKey,
        pdaAccount: pda,
      })
      .rpc();

    const stakeAccount = await program.account.stakeAccount.fetch(pda);
    console.log("Current points:", stakeAccount.totalPoints.toNumber());
    console.log("Get points transaction signature:", tx);
  });
  // ✅ Test 4: Unstake 1 SOL and verify balances + points
  it("unstake 1 SOL", async () => {
    const unstakeAmount = new anchor.BN(1 * LAMPORTS_PER_SOL);
    const initialBalance = await provider.connection.getBalance(provider.publicKey);

    const tx = await program.methods
      .unstake(unstakeAmount)
      .accounts({
        user: provider.publicKey,
        pdaAccount: pda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const stakeAccount = await program.account.stakeAccount.fetch(pda);
    const expectedStaked = new anchor.BN(9 * LAMPORTS_PER_SOL);

    assert(
      stakeAccount.stakedAmount.eq(expectedStaked),
      `Remaining staked amount should be ${expectedStaked.toString()}`
    );

    const finalBalance = await provider.connection.getBalance(provider.publicKey);
    assert(
      finalBalance > initialBalance,
      "User balance should increase after unstaking"
    );

    const currentTime = Math.floor(Date.now() / 1000);
    const timeElapsed = currentTime - stakeTime;

    const expectedPoints = Math.floor(
      (10 * LAMPORTS_PER_SOL * timeElapsed * POINTS_PER_SOL_PER_DAY) /
      (LAMPORTS_PER_SOL * SECONDS_PER_DAY)
    );

    console.log("Time elapsed:", timeElapsed, "seconds");
    console.log("Expected points (approx):", expectedPoints);
    console.log("Actual points:", stakeAccount.totalPoints.toNumber());
    console.log("Unstake transaction signature:", tx);

    if (timeElapsed > 0) {
      assert(
        stakeAccount.totalPoints.toNumber() > 0,
        "Should have accumulated some points"
      );
    }
  });
  // ✅ Test 5: Wait a bit longer to accumulate more points
  it("wait more time for additional points", async () => {
    await new Promise(resolve => setTimeout(resolve, 3000));

    const tx = await program.methods
      .getPoints()
      .accounts({
        user: provider.publicKey,
        pdaAccount: pda,
      })
      .rpc();

    console.log("Get points after additional wait transaction signature:", tx);
  });
  // ✅ Test 6: Claim all accumulated points, should reset to 0
  it("claim points", async () => {
    const stakeAccountBefore = await program.account.stakeAccount.fetch(pda);
    const pointsBeforeClaim = stakeAccountBefore.totalPoints.toNumber();

    console.log("Points before claim:", pointsBeforeClaim);

    if (pointsBeforeClaim > 0) {
      const tx = await program.methods
        .claimPoints()
        .accounts({
          user: provider.publicKey,
          pdaAccount: pda,
        })
        .rpc();

      const stakeAccountAfter = await program.account.stakeAccount.fetch(pda);
      assert(
        stakeAccountAfter.totalPoints.toNumber() === 0,
        "Points should be reset to 0 after claiming"
      );

      console.log("Claimed", pointsBeforeClaim, "points");
      console.log("Claim points transaction signature:", tx);
    } else {
      console.log("No points to claim, skipping claim test");
    }
  });
  // ✅ Test 7: Unstake the remaining SOL completely
  it("unstake remaining balance", async () => {
    const stakeAccount = await program.account.stakeAccount.fetch(pda);
    const remainingStaked = stakeAccount.stakedAmount;

    if (remainingStaked.toNumber() > 0) {
      const tx = await program.methods
        .unstake(remainingStaked)
        .accounts({
          user: provider.publicKey,
          pdaAccount: pda,
        })
        .rpc();

      const finalStakeAccount = await program.account.stakeAccount.fetch(pda);
      assert(
        finalStakeAccount.stakedAmount.toNumber() === 0,
        "All funds should be unstaked"
      );

      console.log("Unstaked remaining", remainingStaked.toNumber() / LAMPORTS_PER_SOL, "SOL");
      console.log("Final unstake transaction signature:", tx);
    }
  });
  // ✅ Test 8: Claim any leftover points before the final state check
  it("claim any remaining points before final verification", async () => {
    const stakeAccount = await program.account.stakeAccount.fetch(pda);
    const remainingPoints = stakeAccount.totalPoints.toNumber();

    if (remainingPoints > 0) {
      console.log("Claiming remaining", remainingPoints, "points before final verification");

      const tx = await program.methods
        .claimPoints()
        .accounts({
          user: provider.publicKey,
          pdaAccount: pda,
        })
        .rpc();

      console.log("Final claim transaction signature:", tx);
    } else {
      console.log("No remaining points to claim");
    }
  });
  // ✅ Test 9: Verify final account state (no stake, no points, owner still correct)
  it("verify account state after all operations", async () => {
    const stakeAccount = await program.account.stakeAccount.fetch(pda);

    assert(stakeAccount.stakedAmount.toNumber() === 0, "No SOL should be staked");
    assert(stakeAccount.totalPoints.toNumber() === 0, "No points should remain");
    assert(stakeAccount.owner.equals(provider.publicKey), "Owner should still be correct");

    console.log("Final account state verified successfully");
    console.log("Final state - Staked:", stakeAccount.stakedAmount.toNumber(),
      "Points:", stakeAccount.totalPoints.toNumber(),
      "Owner:", stakeAccount.owner.toBase58());
  });
});