multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[type_abi]
#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq, Copy, Clone, Debug)]
pub enum State {
    Inactive,
    Active,
}

#[type_abi]
#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq, Copy, Clone, Debug)]
pub enum ListingType {
    FixedPrice,
    Auction,
}

#[type_abi]
#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq, Clone, Debug)]
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

impl<M> Listing<M>
where
    M: ManagedTypeApi
{
    pub fn has_expired(&self, current_time: u64) -> bool {
        if self.listing_type == ListingType::FixedPrice && self.end_time == 0 {
            return false
        }

        self.end_time < current_time
    }

    pub fn has_started(&self, current_time: u64) -> bool {
        if self.start_time > 0 {
            self.start_time >= current_time
        } else {
            true
        }
    }
}

#[type_abi]
#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq, Clone, Debug)]
pub struct Bid<M: ManagedTypeApi> {
    pub id: u64,
    pub listing_id: u64,
    pub bidder: ManagedAddress<M>,
    pub offer: BigUint<M>,
}

#[multiversx_sc::module]
pub trait ConfigModule {
    // state
    #[only_owner]
    #[endpoint(setStateActive)]
    fn set_state_active(&self) {
        self.state().set(State::Active);
    }

    #[only_owner]
    #[endpoint(setStateInactive)]
    fn set_state_inactive(&self) {
        self.state().set(State::Inactive);
    }

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<State>;

    // platform sc address
    #[view(getPlatformAddress)]
    #[storage_mapper("platform_address")]
    fn platform_sc(&self) -> SingleValueMapper<ManagedAddress>;

    // governance token
    #[view(getGovernanceToken)]
    #[storage_mapper("governance_token")]
    fn governance_token(&self) -> SingleValueMapper<TokenIdentifier>;

    // listings
    #[view(getLastListingId)]
    #[storage_mapper("last_listing_id")]
    fn last_listing_id(&self) -> SingleValueMapper<u64>;

    #[view(getListings)]
    #[storage_mapper("listings")]
    fn listings(&self, listing_id: u64) -> SingleValueMapper<Listing<Self::Api>>;

    #[view(getSellerListings)]
    #[storage_mapper("seller_listings")]
    fn seller_listings(&self, seller: ManagedAddress) -> UnorderedSetMapper<u64>;

    // bids
    #[view(getLastBidId)]
    #[storage_mapper("last_bid_id")]
    fn last_bid_id(&self) -> SingleValueMapper<u64>;

    #[view(getBid)]
    #[storage_mapper("bids")]
    fn bids(&self, bid_id: u64) -> SingleValueMapper<Bid<Self::Api>>;

    #[view(getListingBids)]
    #[storage_mapper("listing_bids")]
    fn listing_bids(&self, listing_id: u64) -> UnorderedSetMapper<u64>;

    #[view(getBuyerBids)]
    #[storage_mapper("buyer_bids")]
    fn buyer_bids(&self, bidder: ManagedAddress) -> UnorderedSetMapper<u64>;

    #[view(getBuyerBidsByListingId)]
    fn get_buyer_bids_by_listing_id(&self, buyer: ManagedAddress, listing_id: u64) -> ManagedVec<Bid<Self::Api>> {
        let mut bids = ManagedVec::new();
        for bid_id in self.buyer_bids(buyer).iter() {
            let bid = self.bids(bid_id).get();
            if bid.listing_id == listing_id {
                bids.push(bid);
            }
        }

        bids
    }
}
