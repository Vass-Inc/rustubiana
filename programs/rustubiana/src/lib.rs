use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("FikcqryA4L7H5tHWzGZswWm8HvS19ojuVmZYdHbDRoJJ");

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

        // Transferir NFT para o escrow do leilão
        let cpi_accounts = Transfer {
            from: ctx.accounts.seller_token_account.to_account_info(),
            to: ctx.accounts.auction_token_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, 1)?;

        Ok(())
    }

    pub fn place_bid(ctx: Context<PlaceBid>, amount: u64) -> Result<()> {
        let auction = &mut ctx.accounts.auction;

        require!(!auction.ended, ErrorCode::AuctionEnded);
        require!(
            Clock::get()?.unix_timestamp < auction.end_time,
            ErrorCode::AuctionEnded
        );
        require!(amount >= auction.min_bid, ErrorCode::BidTooLow);
        require!(amount > auction.highest_bid, ErrorCode::BidTooLow);

        let cpi_accounts = Transfer {
            from: ctx.accounts.bidder_token_account.to_account_info(),
            to: ctx.accounts.auction_sol_account.to_account_info(),
            authority: ctx.accounts.bidder.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        auction.highest_bid = amount;
        auction.highest_bidder = Some(*ctx.accounts.bidder.key);

        Ok(())
    }

    pub fn end_auction(ctx: Context<EndAuction>) -> Result<()> {
        let auction = &mut ctx.accounts.auction;

        require!(!auction.ended, ErrorCode::AuctionEnded);
        require!(
            Clock::get()?.unix_timestamp >= auction.end_time,
            ErrorCode::AuctionNotEnded
        );

        auction.ended = true;

        // Fixed: Use the bump from the context accounts, not from ctx.bumps
        let auction_id_bytes = auction.auction_id.to_le_bytes();
        let seeds = &[b"auction", auction_id_bytes.as_ref(), &[ctx.bumps.auction]];
        let signer = &[&seeds[..]];

        if auction.highest_bidder.is_some() {
            let cpi_accounts = Transfer {
                from: ctx.accounts.auction_token_account.to_account_info(),
                to: ctx.accounts.winner_token_account.to_account_info(),
                authority: ctx.accounts.auction.to_account_info(),
            };

            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer,
            );

            token::transfer(cpi_ctx, 1)?;
        } else {
            let cpi_accounts = Transfer {
                from: ctx.accounts.auction_token_account.to_account_info(),
                to: ctx.accounts.seller_token_account.to_account_info(),
                authority: ctx.accounts.auction.to_account_info(),
            };

            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer,
            );

            token::transfer(cpi_ctx, 1)?;
        }

        Ok(())
    }
}

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
    pub nft_mint: Account<'info, Mint>,

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
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct PlaceBid<'info> {
    #[account(mut)]
    pub auction: Account<'info, Auction>,

    #[account(mut)]
    pub bidder: Signer<'info>,

    #[account(mut)]
    pub bidder_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub auction_sol_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct EndAuction<'info> {
    #[account(
        mut,
        seeds = [b"auction", &auction.auction_id.to_le_bytes()],
        bump
    )]
    pub auction: Account<'info, Auction>,

    #[account(mut)]
    pub seller_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"auction_token_account", &auction.auction_id.to_le_bytes()],
        bump
    )]
    pub auction_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub winner_token_account: Account<'info, TokenAccount>,

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

impl Auction {
    pub const LEN: usize = 32 + 32 + 8 + 1 + 32 + 8 + 1 + 8 + 8;
}

#[error_code]
pub enum ErrorCode {
    #[msg("O leilão já terminou")]
    AuctionEnded,
    #[msg("O leilão ainda não terminou")]
    AuctionNotEnded,
    #[msg("Lance muito baixo")]
    BidTooLow,
}
