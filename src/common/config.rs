multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use tfn_platform::common::errors::*;
use tfn_platform::common::config::ProxyTrait as _;

use crate::common::errors::*;

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
            self.start_time <= current_time
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
        require!(!self.platform_sc().is_empty(), ERROR_PLATFORM_NOT_SET);

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

    #[only_owner]
    #[endpoint(setPlatformAddress)]
    fn set_platform_address(&self, platform_sc: ManagedAddress) {
        require!(self.platform_sc().is_empty(), ERROR_PLATFORM_ALREADY_SET);

        self.platform_sc().set(&platform_sc);
        let governance_token = self.platform_contract_proxy()
            .contract(platform_sc)
            .governance_token()
            .execute_on_dest_context::<TokenIdentifier>();
        self.governance_token().set(governance_token);
    }

    // governance token
    #[view(getGovernanceToken)]
    #[storage_mapper("governance_token")]
    fn governance_token(&self) -> SingleValueMapper<TokenIdentifier>;

    // listings
    #[view(getLastListingId)]
    #[storage_mapper("last_listing_id")]
    fn last_listing_id(&self) -> SingleValueMapper<u64>;

    #[view(getListing)]
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

    #[storage_mapper("listing_bids")]
    fn listing_bids(&self, listing_id: u64) -> UnorderedSetMapper<u64>;

    #[storage_mapper("buyer_bids")]
    fn buyer_bids(&self, bidder: &ManagedAddress) -> UnorderedSetMapper<u64>;

    #[view(getLstings)]
    fn get_listings(&self, seller: OptionalValue<ManagedAddress>) -> ManagedVec<Listing<Self::Api>> {
        let mut listings = ManagedVec::new();
        let (all, seller) = match seller {
            OptionalValue::Some(seller) => (false, seller),
            OptionalValue::None => (true, ManagedAddress::zero()),
        };
        for listing_id in 0..self.last_listing_id().get() {
            if self.listings(listing_id).is_empty() {
                continue;
            }

            let listing = self.listings(listing_id).get();
            if all || listing.seller == seller {
                listings.push(listing);
            }
        }

        listings
    }

    #[view(getListingBids)]
    fn get_listing_bids(&self, listing_id: u64) -> ManagedVec<Bid<Self::Api>> {
        let mut bids = ManagedVec::new();
        for bid_id in self.listing_bids(listing_id).iter() {
            let bid = self.bids(bid_id).get();
            bids.push(bid);
        }

        bids
    }

    #[view(getBuyerBids)]
    fn get_buyer_bids(&self, buyer: &ManagedAddress) -> ManagedVec<Bid<Self::Api>> {
        let mut bids = ManagedVec::new();
        for bid_id in self.buyer_bids(buyer).iter() {
            let bid = self.bids(bid_id).get();
            bids.push(bid);
        }

        bids
    }

    #[view(getBuyerBidByListingId)]
    fn get_buyer_bid_by_listing_id(&self, buyer: &ManagedAddress, listing_id: u64) -> Option<Bid<Self::Api>> {
        for bid_id in self.buyer_bids(buyer).iter() {
            let bid = self.bids(bid_id).get();
            if bid.listing_id == listing_id {
                return Some(bid)
            }
        }

        None
    }

    #[view(getListingLastBid)]
    fn get_listing_last_bid(&self, listing_id: u64) -> (BigUint, Option<Bid<Self::Api>>) {
        if self.listings(listing_id).is_empty() {
            sc_panic!(ERROR_LISTING_NOT_FOUND);
        }

        let listing = self.listings(listing_id).get();
        if listing.listing_type != ListingType::Auction {
            sc_panic!(ERROR_NOT_AUCTION);
        }

        let mut highest_bid_amount = listing.min_bid - 1u64;
        let mut highest_bid = None;
        for bid_id in self.listing_bids(listing_id).iter() {
            let bid = self.bids(bid_id).get();
            if bid.offer > highest_bid_amount {
                highest_bid_amount = bid.offer.clone();
                highest_bid = Some(bid);
            }
        }

        (highest_bid_amount, highest_bid)
    }

    // proxies
    #[proxy]
    fn platform_contract_proxy(&self) -> tfn_platform::Proxy<Self::Api>;
}
