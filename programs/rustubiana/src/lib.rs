use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("H7ER5jcZJWXP3vtq5BM6GnmhwzpgFdUdKvD813g9iPqE");

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

        // Transfer NFT from seller to auction token account
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

        // Refund previous highest bidder if there was one
        if let Some(prev_pubkey) = auction.highest_bidder {
            require_keys_eq!(ctx.accounts.prev_bidder.key(), prev_pubkey);
            let prev_bid = auction.highest_bid;

            // Transfer from escrow to previous bidder using invoke
            let refund_ix = anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.escrow.key(),
                &ctx.accounts.prev_bidder.key(),
                prev_bid,
            );

            // Get the seeds for the escrow PDA
            let auction_id_bytes = auction.auction_id.to_le_bytes();
            let seeds = &[b"escrow", auction_id_bytes.as_ref(), &[ctx.bumps.escrow]];
            let signer = &[&seeds[..]];

            anchor_lang::solana_program::program::invoke_signed(
                &refund_ix,
                &[
                    ctx.accounts.escrow.to_account_info(),
                    ctx.accounts.prev_bidder.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                signer,
            )?;
        }

        // Transfer bid amount from bidder to escrow using System Program CPI
        let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.bidder.key(),
            &ctx.accounts.escrow.key(),
            amount,
        );

        anchor_lang::solana_program::program::invoke(
            &transfer_ix,
            &[
                ctx.accounts.bidder.to_account_info(),
                ctx.accounts.escrow.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // Update auction state
        auction.highest_bid = amount;
        auction.highest_bidder = Some(*ctx.accounts.bidder.key);

        msg!("escrow owner: {}", ctx.accounts.escrow.owner);

        Ok(())
    }

    pub fn end_auction(ctx: Context<EndAuction>) -> Result<()> {
        let auction = &mut ctx.accounts.auction;

        require!(!auction.ended, ErrorCode::AuctionEnded);
        require!(
            Clock::get()?.unix_timestamp >= auction.end_time,
            ErrorCode::AuctionNotEnded
        );

        let auction_id = auction.auction_id;
        let highest_bidder = auction.highest_bidder;
        let highest_bid = auction.highest_bid;

        auction.ended = true;

        // Get the auction seeds for signing
        let auction_id_bytes = auction_id.to_le_bytes();
        let auction_seeds = &[b"auction", auction_id_bytes.as_ref(), &[ctx.bumps.auction]];
        let auction_signer = &[&auction_seeds[..]];

        if highest_bidder.is_some() {
            // Transfer NFT to winner
            let cpi_accounts = Transfer {
                from: ctx.accounts.auction_token_account.to_account_info(),
                to: ctx.accounts.winner_token_account.to_account_info(),
                authority: ctx.accounts.auction.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                auction_signer,
            );
            token::transfer(cpi_ctx, 1)?;

            // Transfer SOL to seller using System Program
            let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.escrow.key(),
                &ctx.accounts.seller.key(),
                highest_bid,
            );

            // Get the escrow seeds for signing
            let escrow_seeds = &[b"escrow", auction_id_bytes.as_ref(), &[ctx.bumps.escrow]];
            let escrow_signer = &[&escrow_seeds[..]];

            anchor_lang::solana_program::program::invoke_signed(
                &transfer_ix,
                &[
                    ctx.accounts.escrow.to_account_info(),
                    ctx.accounts.seller.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                escrow_signer,
            )?;
        } else {
            // Return NFT to seller if no bids
            let cpi_accounts = Transfer {
                from: ctx.accounts.auction_token_account.to_account_info(),
                to: ctx.accounts.seller_token_account.to_account_info(),
                authority: ctx.accounts.auction.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                auction_signer,
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
        seeds = [b"auction_token_account".as_ref(), auction_id.to_le_bytes().as_ref()],
        bump
    )]
    pub auction_token_account: Account<'info, TokenAccount>,

    #[account(
    mut,
    seeds = [b"escrow", auction_id.to_le_bytes().as_ref()],
    bump
    )]
    /// CHECK: PDA to receive lamports, no init
    pub escrow: AccountInfo<'info>,

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
    pub prev_bidder: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [b"escrow", auction.auction_id.to_le_bytes().as_ref()],
        bump
    )]
    ///CHECK:
    pub escrow: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
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
    pub seller: Signer<'info>,

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

    /// CHECK: This is a PDA used to hold SOL for bids
    #[account(
        mut,
        seeds = [b"escrow", &auction.auction_id.to_le_bytes()],
        bump
    )]
    pub escrow: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
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
    #[msg("Auction Ended!")]
    AuctionEnded,
    #[msg("Auction still active!")]
    AuctionNotEnded,
    #[msg("Bid too low!")]
    BidTooLow,
}
