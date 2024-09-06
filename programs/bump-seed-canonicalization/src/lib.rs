use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, MintTo, Token, TokenAccount},
};

declare_id!("2aUjVCUm1awbmv8aZHZZ2gUzDxoRzFfrFc9CuubDqycn");

pub const DISCRIMINATOR_SIZE: usize = 8;

#[program]
pub mod bump_seed_canonicalization {
    use super::*;

    // Insecure, allows for creation of multiple accounts for given set of seeds
    pub fn create_user_insecure(ctx: Context<CreateUserInsecure>, bump_seed: u8) -> Result<()> {
        let space = DISCRIMINATOR_SIZE + UserInsecure::INIT_SPACE;
        let lamports = Rent::get()?.minimum_balance(space);

        let ix = anchor_lang::solana_program::system_instruction::create_account(
            &ctx.accounts.payer.key(),
            &ctx.accounts.user.key(),
            lamports,
            space as u64,
            &ctx.program_id,
        );

        anchor_lang::solana_program::program::invoke_signed(
            &ix,
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.user.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[&[ctx.accounts.payer.key().as_ref(), &[bump_seed]]],
        )?;

        // Manually serialize the UserInsecure data
        let user_data = UserInsecure {
            auth: ctx.accounts.payer.key(),
            rewards_claimed: false,
        };
        let data = user_data.try_to_vec()?;
        let mut user_account_data = ctx.accounts.user.try_borrow_mut_data()?;
        user_account_data[DISCRIMINATOR_SIZE..].copy_from_slice(&data);

        msg!("User: {}", ctx.accounts.user.key());
        msg!("Auth: {}", user_data.auth);

        Ok(())
    }

    pub fn claim_insecure(ctx: Context<InsecureClaim>, bump_seed: u8) -> Result<()> {
        // Verify the user account address
        let address = Pubkey::create_program_address(
            &[ctx.accounts.payer.key().as_ref(), &[bump_seed]],
            ctx.program_id,
        )
        .unwrap();
        require_keys_eq!(
            address,
            ctx.accounts.user.key(),
            ClaimError::InvalidUserAccount
        );

        // Deserialize the user account data
        let mut user_data =
            UserInsecure::try_from_slice(&ctx.accounts.user.data.borrow()[DISCRIMINATOR_SIZE..])?;

        // Check if rewards have already been claimed
        require!(!user_data.rewards_claimed, ClaimError::AlreadyClaimed);

        // Mint tokens to the user's associated token account
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.user_ata.to_account_info(),
                    authority: ctx.accounts.mint_authority.to_account_info(),
                },
                &[&["mint".as_bytes(), &[ctx.bumps.mint_authority]]],
            ),
            10,
        )?;

        // Mark rewards as claimed
        user_data.rewards_claimed = true;

        // Serialize the updated user data back into the account
        let mut user_account_data = ctx.accounts.user.try_borrow_mut_data()?;
        user_account_data[DISCRIMINATOR_SIZE..].copy_from_slice(&user_data.try_to_vec()?);

        Ok(())
    }

    // Secure instruction to create a user account
    pub fn create_user_secure(ctx: Context<CreateUserSecure>) -> Result<()> {
        ctx.accounts.user.set_inner(UserSecure {
            auth: ctx.accounts.payer.key(),
            bump: ctx.bumps.user,
            rewards_claimed: false,
        });
        Ok(())
    }

    // Secure instruction to claim rewards
    pub fn claim_secure(ctx: Context<SecureClaim>) -> Result<()> {
        // Mint tokens to the user's associated token account
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.user_ata.to_account_info(),
                    authority: ctx.accounts.mint_authority.to_account_info(),
                },
                &[&[b"mint", &[ctx.bumps.mint_authority]]],
            ),
            10,
        )?;

        // Mark rewards as claimed
        ctx.accounts.user.rewards_claimed = true;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateUserInsecure<'info> {
    /// CHECK: This account is intentionally unchecked and initialized in the instruction
    #[account(mut)]
    pub user: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// Account validation struct for securely creating a user account
#[derive(Accounts)]
pub struct CreateUserSecure<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = DISCRIMINATOR_SIZE + UserSecure::INIT_SPACE,
        seeds = [payer.key().as_ref()],
        bump
    )]
    pub user: Account<'info, UserSecure>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InsecureClaim<'info> {
    /// CHECK: This account is manually deserialized in the instruction
    #[account(mut)]
    pub user: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = payer
    )]
    pub user_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    /// CHECK: This is the mint authority PDA, intentionally left unchecked
    #[account(seeds = ["mint".as_bytes()], bump)]
    pub mint_authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// Account validation struct for secure claiming of rewards
#[derive(Accounts)]
pub struct SecureClaim<'info> {
    #[account(
        mut,
        seeds = [payer.key().as_ref()],
        bump = user.bump,
        constraint = !user.rewards_claimed @ ClaimError::AlreadyClaimed,
        constraint = user.auth == payer.key()
    )]
    pub user: Account<'info, UserSecure>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = payer
    )]
    pub user_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    /// CHECK: This is the mint authority PDA, checked by seeds constraint
    #[account(seeds = [b"mint"], bump)]
    pub mint_authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[account]
#[derive(InitSpace)]
pub struct UserInsecure {
    pub auth: Pubkey,
    pub rewards_claimed: bool,
}

// Secure user account structure
#[account]
#[derive(InitSpace)]
pub struct UserSecure {
    pub auth: Pubkey,
    pub bump: u8,
    pub rewards_claimed: bool,
}

// Error codes for the program
#[error_code]
pub enum ClaimError {
    AlreadyClaimed,
    InvalidUserAccount,
}
