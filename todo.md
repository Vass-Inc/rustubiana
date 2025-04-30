# O que fazer animal

# Lógica

## Structs
### Auction
 - seller,
 - mint do NFT,
 - Highest_bid & Highest_bidder, 
 - end_time,
 - State(active: True, ended: False),

## Functions
### init_auction
 - NFT
 - auction_begins
 - auction_ends

### bid
 - is_active,
 - compara current bid com a  highest_bid
 - Reembolsar a bid provisoriamente mais alta!?
 - Atualiza maior bid e bidder

### end_auction
 - time_ended: True or False,
 - transferir NFT do vendedor para o comprador token::transfer,
 - Send (lamports ou tokens),
 - Marcar leilão como terminado,

### security
 - Proteções contra reentrância,
 - Só o vendedor pode terminar o leilão antes do tempo acabar(why!? finish)


## tests/lib.rs
 - Iniciar o leilão, fazer bids e fechar leilão
 - "teste do performance"


