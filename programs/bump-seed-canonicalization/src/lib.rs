use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod bump_seed_canonicalization {
    use super::*;

    // insecure, allows for creation of multiple accounts for given set of seeds
    pub fn initialize(ctx: Context<Initialize>, bump_seed: u8) -> Result<()> {
        let space = 32;
        let lamports = Rent::get()?.minimum_balance(space as usize);

        let ix = anchor_lang::solana_program::system_instruction::create_account(
            &ctx.accounts.payer.key(),
            &ctx.accounts.pda.key(),
            lamports,
            space,
            &ctx.program_id,
        );

        anchor_lang::solana_program::program::invoke_signed(
            &ix,
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.pda.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[&[&[bump_seed]]],
        )?;

        let mut account = User::try_from_slice(&ctx.accounts.pda.data.borrow()).unwrap();

        account.user = ctx.accounts.payer.key();
        account.serialize(&mut *ctx.accounts.pda.data.borrow_mut())?;

        msg!("PDA: {}", ctx.accounts.pda.key());
        msg!("User: {}", account.user);

        Ok(())
    }

    pub fn insecure(ctx: Context<Unchecked>, bump_seed: u8) -> Result<()> {
        let address = Pubkey::create_program_address(&[&[bump_seed]], ctx.program_id).unwrap();
        if address != ctx.accounts.pda.key() {
            return Err(ProgramError::InvalidArgument.into());
        }

        let account = User::try_from_slice(&ctx.accounts.pda.data.borrow()).unwrap();

        msg!("PDA: {}", ctx.accounts.pda.key());
        msg!("User: {}", account.user);
        Ok(())
    }

    pub fn initialize_with_anchor(ctx: Context<InitializeAnchor>, value: u64) -> Result<()> {
        ctx.accounts.data.value = value;
        // store the bump on the account
        ctx.accounts.data.bump = *ctx.bumps.get("data").unwrap();
        Ok(())
    }

    pub fn verify_address(ctx: Context<VerifyAddress>) -> Result<()> {
        msg!("PDA confirmed to be derived with canonical bump: {}", ctx.accounts.data.key());
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    /// CHECK:
    pub pda: AccountInfo<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// initialize account at PDA via Anchor constraints
#[derive(Accounts)]
pub struct InitializeAnchor<'info> {
        #[account(mut)]
        payer: Signer<'info>,
        #[account(
                init,
                seeds = [DATA_PDA_SEED.as_bytes()],
                // derives the PDA using the canonical bump
                bump,
                payer = payer,
                space = 8 + 8 + 1
            )]
        data: Account<'info, Data>,
        system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct VerifyAddress<'info> {
    #[account(
			seeds = [DATA_PDA_SEED.as_bytes()],
            // guranteed to be the canonical bump every time
			bump = data.bump
		)]
    data: Account<'info, Data>,
}

#[derive(Accounts)]
pub struct Unchecked<'info> {
    /// CHECK:
    pda: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Checked<'info> {
    #[account(seeds = [], bump)]
    pda: Account<'info, UserAccount>,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct User {
    user: Pubkey,
}

// Anchor account
#[account]
pub struct Data {
    value: u64,
    // bump field
    bump: u8
}

/* ************************************************************ */

pub const DATA_PDA_SEED: &str = "test-seed";

#[derive(Clone)]
pub struct UserAccount(User);

impl anchor_lang::Owner for UserAccount {
    fn owner() -> Pubkey {
        ID
    }
}

impl anchor_lang::AccountSerialize for UserAccount {}

impl anchor_lang::AccountDeserialize for UserAccount {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        User::deserialize(buf).map(Self).map_err(Into::into)
    }

    fn try_deserialize(buf: &mut &[u8]) -> Result<Self> {
        Self::try_deserialize_unchecked(buf)
    }
}