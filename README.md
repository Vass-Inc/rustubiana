# Rustubiana — Smart Contract on Solana for NFT Auction

Rustubiana is a Solana smart contract that enables NFT auctions with secure bidding, escrow management. 
This program is written in Rust using the Anchor framework.

---
## Requirements

### Install Rust + Solana + Anchor **

```bash
curl --proto '=https' --tlsv1.2 -sSfL https://solana-install.solana.workers.dev | bash
```

### Run the following command to reload your PATH environment variable to include Cargo's bin directory:

```bash
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
```

### Run the following command to add Solana to your PATH environment variable:

```bash
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
```
## How It Works

### Auction Flow

1. **Auction Creation**  
   Seller initializes an auction and transfers an NFT into a program-controlled escrow account.

2. **Bidding**  
   Bidders place bids that must be higher than the current top bid. Each new highest bid:
   - Is locked in escrow.
   - Refunds the previous highest bidder automatically.

3. **Auction End**  
   When the auction ends (based on `end_time`), the NFT is transferred to the highest bidder, and the seller receives the winning bid.

4. **No Bids?**  
   If no bids were placed, the NFT is returned to the seller.

---

## Project Structure

- `programs/rustubiana/` – Solana smart contract (Anchor)
- `tests/rustubiana.ts` – Mocha tests (TypeScript)
- `lib.rs` – Core smart contract logic
- `rustubiana.ts` – Test suite and NFT setup

---

## Testing

### Local Test Validator (No NFT)

Run tests without NFT functionality (Metaplex program not included by default):

```bash
anchor test
