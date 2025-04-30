use anchor_lang::prelude::*;

declare_id!("6jm6mnCoAMJe4ZbvoBXfeiNJ1Bb8kz29Yq2HsQBBzezQ");

#[program]
pub mod rustubiana {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, data: u64) -> Result<()> {
        ctx.accounts.new_account.data = data;
        msg!("Changed data to: {}", data);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = signer, space = 8 + 8)]
    pub new_account: Account<'info, NewAccount>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub struct Context<'a, 'b, 'c, 'info, T: Bumps> {
    pub program_id: &'a Pubkey,

    pub accounts: &'b mut T,

    pub remaining_accoutns: &'c [AccountInfo<'info>],

    pub bumps: T::Bumps,
}
