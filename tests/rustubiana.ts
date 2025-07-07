import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Rustubiana } from "../target/types/rustubiana";
import {
    Keypair,
    LAMPORTS_PER_SOL,
    PublicKey,
    SystemProgram,
    Transaction,
} from "@solana/web3.js";
import {
    createMint,
    createAssociatedTokenAccount,
    mintTo,
    getOrCreateAssociatedTokenAccount,
    TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { assert } from "chai";
import * as fs from "fs";
import { Metaplex, keypairIdentity, irysStorage } from "@metaplex-foundation/js";

describe("rustubiana", () => {
    anchor.setProvider(anchor.AnchorProvider.env());

    const program = anchor.workspace.Rustubiana as Program<Rustubiana>;
    const provider = anchor.AnchorProvider.env();

    const authority = Keypair.generate();
    const bidder1 = Keypair.generate();
    const bidder2 = Keypair.generate();

    let nftMint: PublicKey;
    let sellerTokenAccount: PublicKey;
    let auctionTokenAccount: PublicKey;
    let bidder1TokenAccount: PublicKey;
    let bidder2TokenAccount: PublicKey;

    const auctionId = new anchor.BN(1);
    const minBid = new anchor.BN(1 * LAMPORTS_PER_SOL);
    const duration = new anchor.BN(3); // Shorten for test

    let auction: PublicKey;
    let escrow: PublicKey;
    let auctionTokenAccountBump: number;

    before(async () => {
        await provider.connection.requestAirdrop(authority.publicKey, 20 * LAMPORTS_PER_SOL);
        await provider.connection.requestAirdrop(bidder1.publicKey, 10 * LAMPORTS_PER_SOL);
        await provider.connection.requestAirdrop(bidder2.publicKey, 10 * LAMPORTS_PER_SOL);

        await new Promise(resolve => setTimeout(resolve, 2000));

        const balance = await provider.connection.getBalance(authority.publicKey);
        console.log("Authority balance:", balance / LAMPORTS_PER_SOL, "SOL");

        nftMint = await createMint(provider.connection, authority, authority.publicKey, null, 0);
        sellerTokenAccount = await createAssociatedTokenAccount(provider.connection, authority, nftMint, authority.publicKey);
        bidder1TokenAccount = await createAssociatedTokenAccount(provider.connection, bidder1, nftMint, bidder1.publicKey);
        bidder2TokenAccount = await createAssociatedTokenAccount(provider.connection, bidder2, nftMint, bidder2.publicKey);

        await mintTo(provider.connection, authority, nftMint, sellerTokenAccount, authority, 1);

        const metaplex = Metaplex.make(provider.connection)
            .use(keypairIdentity(authority))
            .use(irysStorage());

        try {
            if (fs.existsSync("/home/pedro/projetos/rustubiana/tests/release.jpeg")) {
                const imageBuffer = fs.readFileSync("/home/pedro/projetos/rustubiana/tests/release.jpeg");
                const imageUri = await metaplex.storage().upload(imageBuffer);
                await metaplex.nfts().create({
                    uri: imageUri,
                    name: "Release Photo NFT",
                    description: "A photo NFT for auction testing",
                    sellerFeeBasisPoints: 500,
                    mint: nftMint,
                    payer: authority,
                    maxSupply: 0,
                });
            } else {
                console.log("release.jpeg not found, skipping metadata creation");
            }
        } catch (error) {
            console.log("Metaplex metadata creation failed (optional):", error.message);
        }

        [auction] = PublicKey.findProgramAddressSync([
            Buffer.from("auction"),
            auctionId.toBuffer("le", 8),
        ], program.programId);

        [escrow] = PublicKey.findProgramAddressSync([
            Buffer.from("escrow"),
            auctionId.toBuffer("le", 8),
        ], program.programId);

        [auctionTokenAccount, auctionTokenAccountBump] = PublicKey.findProgramAddressSync([
            Buffer.from("auction_token_account"),
            auctionId.toBuffer("le", 8),
        ], program.programId);

        await program.methods.createAuction(auctionId, minBid, duration).accountsPartial({
            auction: auction,
            nftMint: nftMint,
            sellerTokenAccount: sellerTokenAccount,
            auctionTokenAccount: auctionTokenAccount,
            escrow: escrow,
            authority: authority.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        }).signers([authority]).rpc();

        console.log("Authority balance:", balance / LAMPORTS_PER_SOL, "SOL");
    });

    it("Places a bid", async () => {
        const bidAmount = new anchor.BN(2 * LAMPORTS_PER_SOL);

        await program.methods.placeBid(bidAmount).accounts({
            auction,
            bidder: bidder1.publicKey,
            escrow,
            prevBidder: bidder1.publicKey,
            systemProgram: SystemProgram.programId,
        }).signers([bidder1]).rpc();

        const auctionAccount = await program.account.auction.fetch(auction);
        assert.equal(auctionAccount.highestBid.toString(), bidAmount.toString());
        assert.equal(auctionAccount.highestBidder.toString(), bidder1.publicKey.toString());

        const escrowBalance = await provider.connection.getBalance(escrow);
        assert.isTrue(Math.abs(escrowBalance - bidAmount.toNumber()) < 0.01 * LAMPORTS_PER_SOL);
    });

    it("Places a higher bid", async () => {
        const higherBidAmount = new anchor.BN(3 * LAMPORTS_PER_SOL);

        await program.methods.placeBid(higherBidAmount).accounts({
            auction,
            bidder: bidder2.publicKey,
            escrow,
            prevBidder: bidder1.publicKey,
            systemProgram: SystemProgram.programId,
        }).signers([bidder2]).rpc();

        const updatedAuctionAccount = await program.account.auction.fetch(auction);
        assert.equal(updatedAuctionAccount.highestBid.toString(), higherBidAmount.toString());
        assert.equal(updatedAuctionAccount.highestBidder.toString(), bidder2.publicKey.toString());

        const escrowBalance = await provider.connection.getBalance(escrow);
        assert.isTrue(Math.abs(escrowBalance - higherBidAmount.toNumber()) < 0.01 * LAMPORTS_PER_SOL);
    });

    it("Ends the auction and pays out seller", async () => {
        await new Promise(resolve => setTimeout(resolve, 3500));

        const sellerStartBalance = await provider.connection.getBalance(authority.publicKey);

        const winnerTokenAccount = await getOrCreateAssociatedTokenAccount(
            provider.connection,
            bidder2,
            nftMint,
            bidder2.publicKey
        );

        await program.methods.endAuction().accounts({
            auction,
            seller: authority.publicKey,
            sellerTokenAccount,
            auctionTokenAccount,
            winnerTokenAccount: winnerTokenAccount.address,
            escrow,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
        }).signers([authority]).rpc();

        const sellerFinalBalance = await provider.connection.getBalance(authority.publicKey);
        const diff = sellerFinalBalance - sellerStartBalance;

        const auctionAccount = await program.account.auction.fetch(auction);

        assert.isTrue(diff >= auctionAccount.highestBid.toNumber());

        const winnerBalance = await provider.connection.getTokenAccountBalance(winnerTokenAccount.address);
        assert.equal(winnerBalance.value.uiAmount, 1);
    });
});
