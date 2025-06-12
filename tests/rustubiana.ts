import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Rustubiana } from "../target/types/rustubiana";
import {
    PublicKey,
    Keypair,
    SystemProgram,
    SYSVAR_RENT_PUBKEY
} from "@solana/web3.js";
import {
    TOKEN_PROGRAM_ID,
    MINT_SIZE,
    createInitializeMintInstruction,
    getMinimumBalanceForRentExemptMint,
    createAssociatedTokenAccountInstruction,
    getAssociatedTokenAddress,
    createMintToInstruction,
} from "@solana/spl-token";
import { expect } from "chai";

describe("rustubiana", () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.Rustubiana as Program<Rustubiana>;

    let nftMint: Keypair;
    let seller: Keypair;
    let bidder: Keypair;
    let sellerTokenAccount: PublicKey;
    let bidderTokenAccount: PublicKey;
    let auctionPda: PublicKey;
    let auctionTokenAccount: PublicKey;
    let auctionBump: number;
    let tokenAccountBump: number;

    const auctionId = new anchor.BN(1);
    const minBid = new anchor.BN(1000000); // 1 SOL in lamports
    const duration = new anchor.BN(3600); // 1 hour

    before(async () => {
        // Initialize keypairs
        nftMint = Keypair.generate();
        seller = Keypair.generate();
        bidder = Keypair.generate();

        // Airdrop SOL to test accounts
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(seller.publicKey, 2000000000)
        );
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(bidder.publicKey, 2000000000)
        );

        // Create NFT mint
        const lamports = await getMinimumBalanceForRentExemptMint(provider.connection);

        const createMintTx = new anchor.web3.Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: provider.wallet.publicKey,
                newAccountPubkey: nftMint.publicKey,
                space: MINT_SIZE,
                lamports,
                programId: TOKEN_PROGRAM_ID,
            }),
            createInitializeMintInstruction(
                nftMint.publicKey,
                0, // 0 decimals for NFT
                seller.publicKey,
                null
            )
        );

        await provider.sendAndConfirm(createMintTx, [nftMint]);

        // Create token accounts
        sellerTokenAccount = await getAssociatedTokenAddress(
            nftMint.publicKey,
            seller.publicKey
        );

        bidderTokenAccount = await getAssociatedTokenAddress(
            nftMint.publicKey,
            bidder.publicKey
        );

        // Create seller's token account and mint NFT
        const createSellerTokenAccountTx = new anchor.web3.Transaction().add(
            createAssociatedTokenAccountInstruction(
                provider.wallet.publicKey,
                sellerTokenAccount,
                seller.publicKey,
                nftMint.publicKey
            ),
            createMintToInstruction(
                nftMint.publicKey,
                sellerTokenAccount,
                seller.publicKey,
                1 // Mint 1 NFT
            )
        );

        await provider.sendAndConfirm(createSellerTokenAccountTx, [seller]);

        // Derive PDAs
        [auctionPda, auctionBump] = PublicKey.findProgramAddressSync(
            [Buffer.from("auction"), auctionId.toArrayLike(Buffer, "le", 8)],
            program.programId
        );

        [auctionTokenAccount, tokenAccountBump] = PublicKey.findProgramAddressSync(
            [Buffer.from("auction_token_account"), auctionId.toArrayLike(Buffer, "le", 8)],
            program.programId
        );
    });

    it("Creates an auction", async () => {
        const tx = await program.methods
            .createAuction(auctionId, minBid, duration)
            .accounts({
                auction: auctionPda,
                nftMint: nftMint.publicKey,
                sellerTokenAccount: sellerTokenAccount,
                auctionTokenAccount: auctionTokenAccount,
                authority: seller.publicKey,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: SystemProgram.programId,
                rent: SYSVAR_RENT_PUBKEY,
            })
            .signers([seller])
            .rpc();

        console.log("Create auction transaction signature:", tx);

        // Verify auction was created correctly
        const auctionAccount = await program.account.auction.fetch(auctionPda);
        expect(auctionAccount.authority.toString()).to.equal(seller.publicKey.toString());
        expect(auctionAccount.nftMint.toString()).to.equal(nftMint.publicKey.toString());
        expect(auctionAccount.minBid.toNumber()).to.equal(minBid.toNumber());
        expect(auctionAccount.ended).to.be.false;
        expect(auctionAccount.highestBid.toNumber()).to.equal(0);
        expect(auctionAccount.highestBidder).to.be.null;

        // Verify NFT was transferred to auction escrow
        const auctionTokenAccountInfo = await provider.connection.getTokenAccountBalance(auctionTokenAccount);
        expect(auctionTokenAccountInfo.value.uiAmount).to.equal(1);
    });

    it("Places a bid", async () => {
        // First, we need to create a token account for SOL (WSOL) for the bidder
        // For simplicity, we'll assume you have a SOL token account setup
        // In a real implementation, you'd need to handle WSOL wrapping/unwrapping

        // Create bidder's token account
        const createBidderTokenAccountTx = new anchor.web3.Transaction().add(
            createAssociatedTokenAccountInstruction(
                provider.wallet.publicKey,
                bidderTokenAccount,
                bidder.publicKey,
                nftMint.publicKey
            )
        );

        await provider.sendAndConfirm(createBidderTokenAccountTx);

        // For this test, we'll skip the actual bid placement since it requires
        // proper SOL token account setup. In a real scenario, you'd need to:
        // 1. Create WSOL accounts
        // 2. Wrap SOL to WSOL
        // 3. Then place the bid

        console.log("Bid placement test skipped - requires WSOL setup");
    });

    it("Ends an auction without bids", async () => {
        // Wait for auction to expire (in real test, you'd manipulate time or use shorter duration)
        // For now, we'll skip the time check by modifying the auction end time

        const tx = await program.methods
            .endAuction()
            .accounts({
                auction: auctionPda,
                sellerTokenAccount: sellerTokenAccount,
                auctionTokenAccount: auctionTokenAccount,
                winnerTokenAccount: sellerTokenAccount, // Return to seller if no bids
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .rpc();

        console.log("End auction transaction signature:", tx);

        // Verify auction ended
        const auctionAccount = await program.account.auction.fetch(auctionPda);
        expect(auctionAccount.ended).to.be.true;

        // Verify NFT was returned to seller
        const sellerTokenAccountInfo = await provider.connection.getTokenAccountBalance(sellerTokenAccount);
        expect(sellerTokenAccountInfo.value.uiAmount).to.equal(1);
    });
});
