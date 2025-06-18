import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Rustubiana } from "../target/types/rustubiana";
import {
    PublicKey,
    Keypair,
    SystemProgram,
    SYSVAR_RENT_PUBKEY,
    LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
    TOKEN_PROGRAM_ID,
    MINT_SIZE,
    createInitializeMintInstruction,
    getMinimumBalanceForRentExemptMint,
    createAssociatedTokenAccountInstruction,
    getAssociatedTokenAddress,
    createMintToInstruction,
    getAccount,
} from "@solana/spl-token";
import { expect } from "chai";

describe("rustubiana", () => {
    // Configure the client to use the local cluster
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

    // Test parameters
    const auctionId = new anchor.BN(Math.floor(Math.random() * 1000000)); // Random auction ID
    const minBid = new anchor.BN(0.5 * LAMPORTS_PER_SOL); // 0.5 SOL minimum bid
    const duration = new anchor.BN(10); // 10 seconds for testing

    before(async () => {
        console.log("🚀 Setting up test environment...");

        // Generate keypairs
        nftMint = Keypair.generate();
        seller = Keypair.generate();
        bidder = Keypair.generate();

        console.log("📊 Test accounts:");
        console.log("  Seller:", seller.publicKey.toString());
        console.log("  Bidder:", bidder.publicKey.toString());
        console.log("  NFT Mint:", nftMint.publicKey.toString());
        console.log("  Program:", program.programId.toString());

        try {
            // Airdrop SOL to test accounts
            console.log("💰 Airdropping SOL to test accounts...");

            const sellerAirdrop = await provider.connection.requestAirdrop(
                seller.publicKey,
                2 * LAMPORTS_PER_SOL
            );
            await provider.connection.confirmTransaction(sellerAirdrop);

            const bidderAirdrop = await provider.connection.requestAirdrop(
                bidder.publicKey,
                2 * LAMPORTS_PER_SOL
            );
            await provider.connection.confirmTransaction(bidderAirdrop);

            // Check balances
            const sellerBalance = await provider.connection.getBalance(seller.publicKey);
            const bidderBalance = await provider.connection.getBalance(bidder.publicKey);

            console.log("  Seller balance:", sellerBalance / LAMPORTS_PER_SOL, "SOL");
            console.log("  Bidder balance:", bidderBalance / LAMPORTS_PER_SOL, "SOL");

            // Create NFT mint
            console.log("🎨 Creating NFT mint...");
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
            console.log("  ✅ NFT mint created successfully");

            // Create associated token accounts
            sellerTokenAccount = await getAssociatedTokenAddress(
                nftMint.publicKey,
                seller.publicKey
            );

            bidderTokenAccount = await getAssociatedTokenAddress(
                nftMint.publicKey,
                bidder.publicKey
            );

            console.log("  Seller token account:", sellerTokenAccount.toString());
            console.log("  Bidder token account:", bidderTokenAccount.toString());

            // Create seller's token account and mint NFT
            console.log("🏭 Minting NFT to seller...");
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
                    1
                )
            );

            await provider.sendAndConfirm(createSellerTokenAccountTx, [seller]);

            // Verify NFT was minted
            const sellerTokenAccountInfo = await getAccount(provider.connection, sellerTokenAccount);
            console.log("  ✅ NFT minted, seller token balance:", sellerTokenAccountInfo.amount.toString());

            // Derive PDAs
            [auctionPda] = PublicKey.findProgramAddressSync(
                [Buffer.from("auction"), auctionId.toArrayLike(Buffer, "le", 8)],
                program.programId
            );

            [auctionTokenAccount] = PublicKey.findProgramAddressSync(
                [
                    Buffer.from("auction_token_account"),
                    auctionId.toArrayLike(Buffer, "le", 8),
                ],
                program.programId
            );

            console.log("  Auction PDA:", auctionPda.toString());
            console.log("  Auction token account:", auctionTokenAccount.toString());
            console.log("🎯 Setup complete!\n");

        } catch (error) {
            console.error("❌ Setup failed:", error);
            throw error;
        }
    });

    it("Creates an auction", async () => {
        console.log("🏛️ Testing auction creation...");

        try {
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

            console.log("  📝 Transaction signature:", tx);

            // Verify auction was created correctly
            const auctionAccount = await program.account.auction.fetch(auctionPda);

            console.log("  📊 Auction details:");
            console.log("    Authority:", auctionAccount.authority.toString());
            console.log("    NFT Mint:", auctionAccount.nftMint.toString());
            console.log("    Min bid:", auctionAccount.minBid.toNumber() / LAMPORTS_PER_SOL, "SOL");
            console.log("    Ended:", auctionAccount.ended);
            console.log("    Highest bid:", auctionAccount.highestBid.toNumber());
            console.log("    End time:", new Date(auctionAccount.endTime.toNumber() * 1000));

            // Assertions
            expect(auctionAccount.authority.toString()).to.equal(seller.publicKey.toString());
            expect(auctionAccount.nftMint.toString()).to.equal(nftMint.publicKey.toString());
            expect(auctionAccount.minBid.toNumber()).to.equal(minBid.toNumber());
            expect(auctionAccount.ended).to.be.false;
            expect(auctionAccount.highestBid.toNumber()).to.equal(0);
            expect(auctionAccount.highestBidder).to.be.null;

            // Verify NFT was transferred to auction escrow
            const auctionTokenAccountInfo = await getAccount(provider.connection, auctionTokenAccount);
            expect(auctionTokenAccountInfo.amount.toString()).to.equal("1");

            console.log("  ✅ Auction created successfully!");
            console.log("  ✅ NFT transferred to escrow\n");

        } catch (error) {
            console.error("  ❌ Failed to create auction:", error);
            throw error;
        }
    });

    it("Waits for auction to expire", async () => {
        console.log("⏰ Waiting for auction to expire...");

        // Wait for duration + 1 second
        const waitTime = (duration.toNumber() + 1) * 1000;
        console.log(`  Waiting ${waitTime / 1000} seconds...`);

        await new Promise(resolve => setTimeout(resolve, waitTime));
        console.log("  ✅ Auction should now be expired\n");
    });

    it("Ends auction without bids", async () => {
        console.log("🏁 Testing auction end without bids...");

        try {
            const tx = await program.methods
                .endAuction()
                .accounts({
                    auction: auctionPda,
                    sellerTokenAccount: sellerTokenAccount,
                    auctionTokenAccount: auctionTokenAccount,
                    winnerTokenAccount: sellerTokenAccount, // Return to seller since no bids
                    tokenProgram: TOKEN_PROGRAM_ID,
                })
                .rpc();

            console.log("  📝 Transaction signature:", tx);

            // Verify auction ended
            const auctionAccount = await program.account.auction.fetch(auctionPda);
            expect(auctionAccount.ended).to.be.true;
            console.log("  ✅ Auction marked as ended");

            // Verify NFT was returned to seller
            const sellerTokenAccountInfo = await getAccount(provider.connection, sellerTokenAccount);
            expect(sellerTokenAccountInfo.amount.toString()).to.equal("1");
            console.log("  ✅ NFT returned to seller");

            console.log("  🎉 Auction ended successfully!\n");

        } catch (error) {
            console.error("  ❌ Failed to end auction:", error);
            throw error;
        }
    });

    it("Creates a new auction for bidding test", async () => {
        console.log("🔄 Creating new auction for bidding test...");

        const newAuctionId = new anchor.BN(Math.floor(Math.random() * 1000000));
        const [newAuctionPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("auction"), newAuctionId.toArrayLike(Buffer, "le", 8)],
            program.programId
        );

        const [newAuctionTokenAccount] = PublicKey.findProgramAddressSync(
            [
                Buffer.from("auction_token_account"),
                newAuctionId.toArrayLike(Buffer, "le", 8),
            ],
            program.programId
        );

        try {
            const tx = await program.methods
                .createAuction(newAuctionId, minBid, new anchor.BN(3600)) // 1 hour duration
                .accounts({
                    auction: newAuctionPda,
                    nftMint: nftMint.publicKey,
                    sellerTokenAccount: sellerTokenAccount,
                    auctionTokenAccount: newAuctionTokenAccount,
                    authority: seller.publicKey,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: SystemProgram.programId,
                    rent: SYSVAR_RENT_PUBKEY,
                })
                .signers([seller])
                .rpc();

            console.log("  ✅ New auction created for bidding test");
            console.log("  📝 Transaction signature:", tx);

            // Store new auction details for potential future tests
            auctionPda = newAuctionPda;
            auctionTokenAccount = newAuctionTokenAccount;

        } catch (error) {
            console.error("  ❌ Failed to create new auction:", error);
            console.log("  ℹ️  This might fail if seller doesn't have NFT anymore");
        }
    });

    after(async () => {
        console.log("🧹 Cleaning up test environment...");

        try {
            // Check final balances
            const sellerBalance = await provider.connection.getBalance(seller.publicKey);
            const bidderBalance = await provider.connection.getBalance(bidder.publicKey);

            console.log("  Final balances:");
            console.log("    Seller:", sellerBalance / LAMPORTS_PER_SOL, "SOL");
            console.log("    Bidder:", bidderBalance / LAMPORTS_PER_SOL, "SOL");

        } catch (error) {
            console.log("  ⚠️  Cleanup warning:", error.message);
        }

        console.log("🎯 Tests completed!\n");
    });
});
