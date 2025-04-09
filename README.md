<p align="center">
  <a href="https://teachfi.network/" target="blank"><img src="https://teachfi.network/teachfi-logo.svg" width="256" alt="TeachFi Logo" /><br/>NFT Marketplace</a>
</p>
<br/>
<br/>
<br/>

# Description

This is a child contract of Platform SC. A separate instance is deployed for each platform subscriber.
<br/>
<br/>
<br/>
## Endpoints

<br/>

```rust
addListing(
    listing_type: ListingType,
    price: BigUint,
    min_bid: BigUint,
    buyout_price: BigUint,
    start_time: u64,
    end_time: u64,
) -> u64
```
>[!IMPORTANT]
>*Requirements:* state = active.

>[!NOTE]
>Lists a NFT on the marketplace. If the listing type is Auction, the `end_time` parameter is mandatory. Returns the ID of the newly craeted Listing object.
<br/>

```rust
remove_listing(listing_id: u64)
```
>[!IMPORTANT]
>*Requirements:* state = active, caller = listing.seller or caller = owner.

>[!NOTE]
>Removes the NFT listing from the marketplace and if the type is Auction, it also removes all bids and reimburses the bidders.
<br/>

```rust
buy(listing_id: u64)
```
>[!IMPORTANT]
>*Requirements:* state = active, payment must be made in DAO's governance tokens. If the listing type is Auction, the payment amount must be equal to the listing's buyout price.

>[!NOTE]
>Sends the NFT to the buyer and the payment amount to the seller, then removes the listing.
<br/>

```rust
addBid(listing_id: u64)
```
>[!IMPORTANT]
>*Requirements:* state = active, listing type must be Auction, payment must be in DAO's governance tokens and amount > last bid.

>[!NOTE]
>Places a bid for the specified `listing_id`. If the bidder already has a bid on `listing_id`, the old bid is removed. 
>If payment.amount >= buyout_price, the NFT is bought automatically and the difference in tokens is sent back to the caller.
<br/>

```rust
removeBid(bid_id: u64)
```
>[!IMPORTANT]
>*Requirements:* state = active, caller = bidder.

>[!NOTE]
>Removes the bid from the Listing and reimburses the caller with the bidded amount.
<br/>

```rust
acceptBid(listing_id: u64)
```
>[!IMPORTANT]
>*Requirements:* state = active, caller = seller, listing type must be Auction and it must have at least a bid.

>[!NOTE]
>Accepts the highest bid and removes the listing.
<br/>

```rust
setStateActive()
```
>[!IMPORTANT]
*Requirements:* the caller must be the SC owner.

>[!NOTE]
>Sets the SC state as active.
<br/>

```rust
setStateInactive()
```
>[!IMPORTANT]
*Requirements:* the caller must be the SC owner.

>[!NOTE]
>Sets the SC state as inactive.
<br/>

```rust
setPlatformAddress(platform_sc: ManagedAddress)
```
>[!IMPORTANT]
>*Requirements:* caller = owner, platform should be empty.

>[!NOTE]
>Sets the Platform SC address and retrieves the governance token id from it.

<br/>

## View functions

<br/>

```rust
getState() -> State
```
>Returns the state of the SC (Active or Inactive).
<br/>

```rust
getPlatformAddress() -> ManagedAddress
```
>Returns the Platform SC address if set.
<br/>

```rust
getGovernanceToken() -> TokenIdentifier
```
>Returns the DAO's governance token id (only if the Platform SC is set).
<br/>

```rust
getListing(listing_id: u64) -> Listing
```
>Returns the Listing object associated with the `listing_id` parameter.
<br/>

```rust
getLastListingId() -> u64
```
>Returns `ID - 1` of the last added Listing object.
<br/>

```rust
getBid(bid_id: u64) -> Bid
```
>Returns the Bid object associated with the `bid_id` parameter.
<br/>

```rust
getLastBidId() -> u64
```
>Returns `ID - 1` of the last added Bid object.
<br/>

```rust
getListingLastBid(listing_id: u64) -> (BigUint, Option<Bid>)
```
>If the listing does not exist or the type is not Auction, the SC panics. Otherwise, it returns the highest bidded amount and Some(Bid). If no bids are placed, it returns (min_bid - 1, None).
<br/>

```rust
getBuyerBidByListingId(buyer: &ManagedAddress, listing_id: u64) -> Option<Bid>
```
>If the specified `buyer` has placed a bid for `listing_id`, it returns Some(Bid) and None otherwise.

<br/>

## Custom types

<br/>

```rust
pub enum State {
    Inactive,
    Active,
}
```

<br/>

```rust
pub enum ListingType {
    FixedPrice,
    Auction,
}
```

<br/>

```rust
pub struct Listing<M: ManagedTypeApi> {
    pub id: u64,
    pub seller: ManagedAddress<M>,
    pub token_id: TokenIdentifier<M>,
    pub token_nonce: u64,
    pub token_amount: BigUint<M>,
    pub listing_type: ListingType,
    pub price: BigUint<M>,
    pub min_bid: BigUint<M>, // auction only
    pub buyout_price: BigUint<M>, // auction only
    pub start_time: u64, // optional
    pub end_time: u64, // optional for fixed price, mandatory for auction
}
```

<br/>

```rust
pub struct Bid<M: ManagedTypeApi> {
    pub id: u64,
    pub listing_id: u64,
    pub bidder: ManagedAddress<M>,
    pub offer: BigUint<M>,
}
```
