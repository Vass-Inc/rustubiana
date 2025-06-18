# 



# Function Transfer

pub fn transfer<'info> (
    ctx: CpiContext<'_,'_,'_, 'info, Transfer<'info>>,
    lamportes: u64,
) -> Result<()>


# Todos!?

    - Instruction to Bid -> blind bid.
    - Instruction to Store the highest bid and refund the previous highest bid
    - Close the auction when it ends and send the nft to highest_bidder at the end of the auction.
    - to close bid accounts to let bidders recoup their rent spendings.


# Até dia 13

# Auction





# Future endevours?
    -ZPK that the contract knows the highest_bid.
