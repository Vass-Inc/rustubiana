use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};

declare_id!("6jm6mnCoAMJe4ZbvoBXfeiNJ1Bb8kz29Yq2HsQBBzezQ");

#[program]
pub mod rustubiana {
    use super::*;

    pub fn create_auction(
        ctx: Context<CreateAuction>,
        auction_id: u64,
        min_bid: u64,
        duration: i64,
    ) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        auction.authority = *ctx.accounts.authority.key;
        auction.nft_mint = ctx.accounts.nft_mint.key();
        auction.highest_bid = 0;
        auction.highest_bidder = None;
        auction.min_bid = min_bid;
        auction.ended = false;
        auction.end_time = Clock::get()?.unix_timestamp + duration;
        auction.auction_id = auction_id;

        let cpi_accounts = Transfer {
            from: ctx.accounts.seller_token_account.to_account_info(),
            to: ctx.accounts.auction_token_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);

        token::transfer(cpi_ctx, 1)?;

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

// TODO:
// Implementar um PDA Account
//
// Transfer NFT from Owner account to PDA Account
//
//

// FIXME:
// Arranjar a parte do NFT (i don't know anymore at this point)

// Structs

#[derive(Accounts)]
#[instruction(auction_id: u64)]
pub struct CreateAuction<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Auction::LEN,
        seeds = [b"auction".as_ref(), &auction_id.to_le_bytes()],
        bump
    )]
    pub auction: Account<'info, Auction>,

    #[account(mut)]
    pub nft: Account<'info, Mint>,

    #[account(
        mut,
        constraint = seller_token_account.amount >= 1,
        constraint = seller_token_account.mint == nft_mint.key()
    )]
    pub seller_token_account: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = authority,
        token::mint = nft_mint,
        token::authority = auction,
        seeds = [b"auction_token_account".as_ref(), &auction_id.to_le_bytes()],
        bump
    )]
    pub auction_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct PlaceBid<'info> {
    #[account(mut)]
    pub auction: Account<'info, Auction>,

    #[account(mut)]
    pub bidder: Signer<'info>,

    #[account(
        mut,
        constraint = bidder_token_account.owner == bidder.key(),
    )]
    pub bidder_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = auction_sol_account.owner == auction.key(),
    )]
    pub auction_sol_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct EndAuction<'info> {
    #[account(mut, seeds = [b"auction".as_ref(), &auctino.auction_id.to_le_bytes()], bump)]
    pub auction: Account<'info, Auction>,

    #[account(mut)]
    pub seller_token_account: Account<'info, TokenAccount>,

    #[account(mut, constraint = auction_token_account.mint == auction.key())]
    pub auction_token_account: Account<'info, TokenAccount>,

    #[account(mut, constraint = winner_token_account.mint == auction.nft_mint)]
    pub winner: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[account]
pub struct Auction {
    pub authority: Pubkey,
    pub nft_mint: Pubkey,
    pub highest_bid: u64,
    pub highest_bidder: Option<Pubkey>,
    pub min_bid: u64,
    pub ended: bool,
    pub end_time: i64,
    pub auction_id: u64,
}

// Espaço pre-alocado para o contrato
impl Auction {
    pub const LEN: usize = 32 + 32 + 8 + 1 + 32 + 8 + 1 + 8 + 8;
}

#[error_code]
pub enum ErrorCode {
    #[msg("The auction has already ended.")]
    AuctionAlreadyEnded,
    #[msg("The auction still open.")]
    AuctionNotEnded,
    #[msg("Your bid must be higher than the current highest bid.")]
    BidTooLow,
}
