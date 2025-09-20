import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SimpleEscrow } from "../target/types/simple_escrow";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY, Keypair } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";
const { assert } = require("chai");

describe("simple_escrow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SimpleEscrow as Program<SimpleEscrow>;

  let mint: PublicKey;
  let initializerTokenAccount: PublicKey;
  let receiverTokenAccount: PublicKey;
  let escrowKeypair: Keypair;
  let vaultAuthority: PublicKey;
  let vault: PublicKey;
  let receiver: Keypair;

  const escrowAmount = new anchor.BN(100000000); // 100 tokens

  before(async () => {
    // Create mint
    mint = await createMint(
      provider.connection,
      (provider.wallet as any).payer,
      provider.publicKey,
      null,
      6
    );

    // Create initializer's token account
    initializerTokenAccount = await createAccount(
      provider.connection,
      (provider.wallet as any).payer,
      mint,
      provider.publicKey
    );

    // Create receiver keypair
    receiver = Keypair.generate();

    // Create receiver's associated token account
    receiverTokenAccount = await createAccount(
      provider.connection,
      (provider.wallet as any).payer,
      mint,
      receiver.publicKey
    );

    // Mint tokens to initializer
    await mintTo(
      provider.connection,
      (provider.wallet as any).payer,
      mint,
      initializerTokenAccount,
      provider.publicKey,
      1000000000 // 1000 tokens
    );

    // Generate escrow keypair
    escrowKeypair = Keypair.generate();

    // Derive PDA for vault authority
    [vaultAuthority] = await PublicKey.findProgramAddress(
      [Buffer.from("vault"), escrowKeypair.publicKey.toBuffer()],
      program.programId
    );

    // Derive vault token account
    vault = await anchor.utils.token.associatedAddress({
      mint: mint,
      owner: vaultAuthority
    });
  });

  it("Initializes escrow", async () => {
    await program.methods
      .initializeEscrow(escrowAmount, receiver.publicKey)
      .accounts({
        escrow: escrowKeypair.publicKey,
        initializer: provider.publicKey,
        initializerTokenAccount: initializerTokenAccount,
        vaultAuthority: vaultAuthority,
        vault: vault,
        mint: mint,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([escrowKeypair])
      .rpc();

    // Verify escrow state
    const escrowAccount = await program.account.escrow.fetch(escrowKeypair.publicKey);
    assert.ok(escrowAccount.initializer.equals(provider.publicKey));
    assert.ok(escrowAccount.receiver.equals(receiver.publicKey));
    assert.ok(escrowAccount.mint.equals(mint));
    assert.equal(escrowAccount.amount.toString(), escrowAmount.toString());

    // Verify tokens were transferred to vault
    const vaultBalance = await provider.connection.getTokenAccountBalance(vault);
    assert.equal(vaultBalance.value.amount, escrowAmount.toString());
  });

  it("Claims escrow", async () => {
    // Get initial balances
    const initialReceiverBalance = await provider.connection.getTokenAccountBalance(receiverTokenAccount);
    const initialVaultBalance = await provider.connection.getTokenAccountBalance(vault);

    await program.methods
      .claimEscrow()
      .accounts({
        escrow: escrowKeypair.publicKey,
        vaultAuthority: vaultAuthority,
        vault: vault,
        receiver: receiver.publicKey,
        receiverTokenAccount: receiverTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([receiver])
      .rpc();

    // Verify balances after claim
    const finalReceiverBalance = await provider.connection.getTokenAccountBalance(receiverTokenAccount);
    const finalVaultBalance = await provider.connection.getTokenAccountBalance(vault);

    assert.equal(
      finalReceiverBalance.value.amount,
      (BigInt(initialReceiverBalance.value.amount) + BigInt(escrowAmount.toString())).toString()
    );
    assert.equal(
      finalVaultBalance.value.amount,
      (BigInt(initialVaultBalance.value.amount) - BigInt(escrowAmount.toString())).toString()
    );
  });
});
