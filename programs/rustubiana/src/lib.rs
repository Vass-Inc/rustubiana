use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("6jm6mnCoAMJe4ZbvoBXfeiNJ1Bb8kz29Yq2HsQBBzezQ");

#[program]
pub mod rustubiana {
    use anchor_lang::solana_program::clock;

    use super::*;

    pub fn init_auction(
        ctx: Context<InitAuction>,
        _starting_bid: u64,
        _duration: i64,
    ) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        auction.owner = *ctx.accounts.owner.key;
        auction.nft = ctx.accounts.nft.key();
        auction.highest_bid = 0;
        auction.highest_bidder = Pubkey::default();
        auction.end_time = Clock::get()?.unix_timestamp + _duration;
        auction.state = true;

        Ok(())
    }

    pub fn bid(ctx: Context<Bid>, amount: u64) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        let bidder = &ctx.accounts.bidder;
        let now = Clock::get()?.unix_timestamp;
        let (vault_pda, _bump) =
            Pubkey::find_program_address(&[b"Vault", auction.key().as_ref()], &rustubiana::ID);

        require!(now < auction.end_time, ErrorCode::AuctionAlreadyEnded);
        require!(auction.state, ErrorCode::AuctionAlreadyEnded);
        require!(amount > auction.highest_bid, ErrorCode::BidTooLow);

        auction.highest_bid = amount;
        auction.highest_bidder = bidder.key();

        Ok(())
    }

    pub fn end_auction(ctx: Context<EndAuction>) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        let now = Clock::get()?.unix_timestamp;
        require!(now >= auction.end_time, ErrorCode::AuctionAlreadyEnded);
        require!(auction.state, ErrorCode::AuctionAlreadyEnded);

        auction.state = false;

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

#[derive(Accounts)]
pub struct InitAuction<'info> {
    #[account(init, payer = owner, space = 8 + 129)]
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

    pub nft: Account<'info, Mint>,

    #[account(mut)]
    pub vault_pda: UncheckedAccount<'info>,

    #[account(mut)]
    pub previous_highest_bidder: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

// TODO:
// Está isto correcto?

// FIXME:
// Arranjar a parte do NFT (i don't know more than me at this point)

#[derive(Accounts)]
pub struct EndAuction<'info> {
    #[account(mut, has_one = owner, has_one = nft)]
    pub auction: Account<'info, Auction>,

    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut)]
    pub nft_seller: Account<'info, TokenAccount>,

    #[account(mut)]
    pub nft_buyer: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault_pda: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct Auction {
    pub owner: Pubkey,
    pub nft: Pubkey,
    pub highest_bid: u64,
    pub highest_bidder: Pubkey,
    pub end_time: i64,
    pub state: bool,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The auction has already ended.")]
    AuctionAlreadyEnded,
    #[msg("Your bid must be higher than the current highest bid.")]
    BidTooLow,
}
