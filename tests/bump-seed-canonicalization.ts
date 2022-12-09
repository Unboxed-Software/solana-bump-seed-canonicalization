import * as anchor from "@project-serum/anchor"
import { Program } from "@project-serum/anchor"
import {
  findProgramAddressSync,
} from "@project-serum/anchor/dist/cjs/utils/pubkey"
import { expect } from "chai"
import { BumpSeedCanonicalization } from "../target/types/bump_seed_canonicalization"
import { PublicKey, Keypair, SystemProgram } from '@solana/web3.js'
import { safeAirdrop, findNonCanonicalPda } from './utils/utils'
import { BN } from "bn.js"

describe("bump-seed-canonicalization", async () => {
  // Initialize testing environment
  anchor.setProvider(anchor.AnchorProvider.env())
  const provider = anchor.AnchorProvider.env()
  const program = anchor.workspace.BumpSeedCanonicalization as Program<BumpSeedCanonicalization>

  // payer for creation of accounts
  const payer = Keypair.generate()

  // derive first PDA using canonical bump
  const [pda, bump] = findProgramAddressSync([], program.programId)
  console.log("First canonical PDA:", pda.toString())
  console.log("Canonical bump:", bump)

  // derive second PDA using a non-canonical bump
  const [pda_not_canonical, not_canonical_bump] = await findNonCanonicalPda(
    [],
    program.programId
  )
  console.log("First non-canonical PDA:", pda_not_canonical.toString())
  console.log("Non-canonical bump:", not_canonical_bump)


  it("Initialize PDA using canonical bump", async () => {
    await program.methods.initialize(bump).accounts({ pda: pda }).rpc()
  })

  it("Initialize PDA using non-canonical bump", async () => {
    await program.methods
      .initialize(not_canonical_bump)
      .accounts({ pda: pda_not_canonical })
      .rpc()
  })

  it("Verify PDA with canonical bump", async () => {
    await program.methods.insecure(bump).accounts({ pda: pda }).rpc()
  })

  it("Verify PDA with non-canonical bump", async () => {
    await program.methods
      .insecure(not_canonical_bump)
      .accounts({ pda: pda_not_canonical })
      .rpc()
  })

  it("Create new account with canonnical bump via Anchor constraints", async () => {
    // payer needs SOL to pay for account creation
    await safeAirdrop(payer.publicKey, provider.connection)

    const pda_seed = "test-seed"
    const value = 55
    const [pda_canonical, canonical_bump] = await PublicKey.findProgramAddress(
      [Buffer.from(pda_seed)],
      program.programId
    )

    console.log("Second canonical PDA:", pda_canonical.toString())
    console.log("Canonical bump:", canonical_bump)

    await program.methods.initializeWithAnchor(new BN(value))
    .accounts({
      payer: payer.publicKey,
      data: pda_canonical,
      systemProgram: SystemProgram.programId
    })
    .signers([payer])
    .rpc()
  })

  // this test should fail because Anchor will not initalize an account at a PDA that is not derived with the canonical bump
  it("Create new account with non-canonnical bump via Anchor constraints (should fail)", async () => {
    // same seed as test above
    const pda_seed = "test-seed"
    // derive PDA using the same seed as before, but use a different bump that is not the canonical bump
    const [non_canonical_pda, non_canonical_bump] = await findNonCanonicalPda(
      [Buffer.from(pda_seed)],
      program.programId
    )
    console.log("Second non-canonical PDA:", non_canonical_pda.toString())
    console.log("Non-canonical bump:", non_canonical_bump)
    try {
      await program.methods.initializeWithAnchor(new BN(non_canonical_bump))
      .accounts({
        payer: payer.publicKey,
        data: non_canonical_pda,
        systemProgram: SystemProgram.programId
      })
      .signers([payer])
      .rpc()
    } catch (e) {
      console.log(e.message)
      expect(e.message).to.eq("failed to send transaction: Transaction simulation failed: Error processing Instruction 0: Cross-program invocation with unauthorized signer or writable account")
    }
  })

  it("Verify address of account PDA via Anchor constraints with canonical bump", async () => {
    const pda_seed = "test-seed"
    const [pda_canonical, canonical_bump] = await PublicKey.findProgramAddress(
      [Buffer.from(pda_seed)],
      program.programId
    )

    await program.methods.verifyAddress()
    .accounts({
      data: pda_canonical,
    })
    .rpc()
  })

  it("Try to verify address of non-canonical pda (should fail)", async () => {
    const pda_seed = "test-seed"
    const [non_canonical_pda, non_canonical_bump] = await findNonCanonicalPda(
      [Buffer.from(pda_seed)],
      program.programId
    )

    try {
      const createAcctTx = await program.methods.verifyAddress()
      .accounts({
        data: non_canonical_pda,
      })
      .rpc()
      console.log("Verify PDA account via Anchor constraints: ", createAcctTx)
    } catch (e) {
        console.log(e.message)
        expect(e.message).to.eq("AnchorError caused by account: data. Error Code: AccountNotInitialized. Error Number: 3012. Error Message: The program expected this account to be already initialized.")
      }
  })
})
