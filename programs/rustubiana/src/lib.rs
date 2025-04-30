use anchor_lang::{prelude::*, solana_program::example_mocks::solana_account::Account};

declare_id!("6jm6mnCoAMJe4ZbvoBXfeiNJ1Bb8kz29Yq2HsQBBzezQ");

#[account]
pub struct Auction {
    pub owner: PubKey,
    pub nft: Pubkey,
    pub highest_bid: u128,
    pub highest_bidder: Pubkey,
    pub end_time: i64,
    pub state: bool,
}

#[derive(Accounts)]
pub struct Init_auction<'info> {
    #[account(init, payer = owner, space = 8 + 32 + 32 + 16 + 16 + 1)]
    pub auction: Account<'info, Auction>,
    pub nft: Account<'info, Mint>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Bid<'info> {
    #[account(mut, has_one = nft)]
    pub auction: Account<'info, Auction>,
    #[account(mut)]
    pub bidder: Signer<'info>,
    #[account(mut)]
    pub winning_bidder: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct EndAuciton<'info> {
    #[account(mut, has_one = owner, has_one = nft)]
    pub auction: Account<'info, Auction>,

    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut)]
    pub nft_seller: Account<'info, TokenAccount>,

    #[account(mut)]
    pub nft_buyer: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[program]
pub mod rustubiana {
    use super::*;

    pub fn Init_auction(ctx: Context<InitAuction>, starting_bid: u64, duration: i64) -> Result<()> {
    }

    pub fn bid(ctx: Context<Bid>, amount: u64) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        let bidder = &ctx.accounts.bidder;

        if amount > auction.highest_bid {
            auction.highest_bid = amount;
            auction.highest_bidder = *bidder.key;
        }

        Ok(())
    }

    pub fn end_auction(ctx: Context<EndAuction>) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        let owner = &ctx.accounts.owner;

        if auction.state == false {
            return Err("Auction is already ended");
        }

        auction.state = false;

        // Transfer the NFT to the highest bidder
        let cpi_accounts = Transfer {
            from: ctx.accounts.nft_seller.to_account_info(),
            to: ctx.accounts.nft_buyer.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, 1)?;

        Ok(())
    }
}
