import * as anchor from "@project-serum/anchor"
import { Program } from "@project-serum/anchor"
import {
  createProgramAddressSync,
  findProgramAddressSync,
} from "@project-serum/anchor/dist/cjs/utils/pubkey"
import { expect } from "chai"
import { BumpSeedCanonicalization } from "../target/types/bump_seed_canonicalization"
import { Keypair } from "@solana/web3.js"
import { safeAirdrop } from "./utils/utils"
import { BN } from "bn.js"
import {
  createAssociatedTokenAccount,
  createMint,
  getAccount,
  getAssociatedTokenAddress,
} from "@solana/spl-token"

describe("bump-seed-canonicalization", async () => {
  // Initialize testing environment
  let provider, program, payer, mintAuthority, mint

  before(async () => {
    anchor.setProvider(anchor.AnchorProvider.env())
    provider = anchor.AnchorProvider.env()
    program = anchor.workspace
      .BumpSeedCanonicalization as Program<BumpSeedCanonicalization>
    payer = Keypair.generate()
    await safeAirdrop(payer.publicKey, provider.connection)
    ;[mintAuthority] = findProgramAddressSync(
      [Buffer.from("mint")],
      program.programId
    )

    mint = await createMint(
      provider.connection,
      payer,
      mintAuthority,
      mintAuthority,
      0
    )
  })

  it("Attacker can claim more than reward limit with insecure instructions", async () => {
    const attacker = Keypair.generate()
    await safeAirdrop(attacker.publicKey, provider.connection)
    const ataKey = await getAssociatedTokenAddress(mint, attacker.publicKey)

    let numClaims = 0

    for (let i = 0; i < 256; i++) {
      try {
        const pda = createProgramAddressSync(
          [attacker.publicKey.toBuffer(), Buffer.from([i])],
          program.programId
        )
        await program.methods
          .createUserInsecure(i)
          .accounts({
            user: pda,
            payer: attacker.publicKey,
          })
          .signers([attacker])
          .rpc()
        await program.methods
          .claimInsecure(i)
          .accounts({
            user: pda,
            mint,
            payer: attacker.publicKey,
            userAta: ataKey,
          })
          .signers([attacker])
          .rpc()

        numClaims += 1
      } catch (error) {
        if (
          error.message !== "Invalid seeds, address must fall off the curve"
        ) {
          console.log(error)
        }
      }
    }

    const ata = await getAccount(provider.connection, ataKey)

    console.log(
      `Attacker claimed ${numClaims} times and got ${Number(ata.amount)} tokens`
    )

    expect(numClaims).to.be.greaterThan(1)
    expect(Number(ata.amount)).to.be.greaterThan(10)
  })

  it("Attacker can only claim once with secure instructions", async () => {
    const attacker = Keypair.generate()
    await safeAirdrop(attacker.publicKey, provider.connection)
    const ataKey = await getAssociatedTokenAddress(mint, attacker.publicKey)
    const [userPDA] = findProgramAddressSync(
      [attacker.publicKey.toBuffer()],
      program.programId
    )

    await program.methods
      .createUserSecure()
      .accounts({
        payer: attacker.publicKey,
      })
      .signers([attacker])
      .rpc()

    await program.methods
      .claimSecure()
      .accounts({
        payer: attacker.publicKey,
        userAta: ataKey,
        mint,
        user: userPDA,
      })
      .signers([attacker])
      .rpc()

    let numClaims = 1

    for (let i = 0; i < 256; i++) {
      try {
        const pda = createProgramAddressSync(
          [attacker.publicKey.toBuffer(), Buffer.from([i])],
          program.programId
        )
        await program.methods
          .createUserSecure()
          .accounts({
            user: pda,
            payer: attacker.publicKey,
          })
          .signers([attacker])
          .rpc()

        await program.methods
          .claimSecure()
          .accounts({
            payer: attacker.publicKey,
            userAta: ataKey,
            mint,
            user: pda,
          })
          .signers([attacker])
          .rpc()

        numClaims += 1
      } catch {}
    }

    const ata = await getAccount(provider.connection, ataKey)

    expect(Number(ata.amount)).to.equal(10)
    expect(numClaims).to.equal(1)
  })
})
