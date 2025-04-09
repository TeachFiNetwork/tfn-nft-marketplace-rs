#![no_std]

multiversx_sc::imports!();

pub mod common;

use common::{config::*, errors::*};
use tfn_platform::common::config::ProxyTrait as _;

#[multiversx_sc::contract]
pub trait TFNNFTMarketplaceContract<ContractReader>:
    common::config::ConfigModule
{
    #[init]
    fn init(&self) {
        let caller = self.blockchain().get_caller();
        if self.blockchain().is_smart_contract(&caller) {
            self.platform_sc().set(&caller);
            let governance_token = self.platform_contract_proxy()
                .contract(caller)
                .governance_token()
                .execute_on_dest_context::<TokenIdentifier>();
            self.governance_token().set(governance_token);
            self.set_state_active();
        }
    }

    #[upgrade]
    fn upgrade(&self) {
    }

    #[payable("*")]
    #[endpoint(addListing)]
    fn add_listing(
        &self,
        listing_type: ListingType,
        price: BigUint,
        min_bid: BigUint,
        buyout_price: BigUint,
        start_time: u64,
        end_time: u64,
    ) -> u64 {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);

        let caller = self.blockchain().get_caller();
        self.check_whitelisted(&caller);

        let current_time = self.blockchain().get_block_timestamp();
        if listing_type == ListingType::Auction {
            require!(min_bid <= buyout_price, ERROR_WRONG_BIDS);
            require!(min_bid > 0, ERROR_WRONG_BIDS);
        };
        if start_time > 0 {
            require!(start_time > current_time, ERROR_WRONG_TIMES);
        }
        if end_time > 0 {
            require!(end_time > current_time, ERROR_WRONG_TIMES);
        }
        if (start_time > 0 && end_time > 0) || listing_type == ListingType::Auction {
            require!(end_time > start_time, ERROR_WRONG_TIMES);
        }

        let payments = self.call_value().all_esdt_transfers();
        require!(!payments.is_empty(), ERROR_NO_PAYMENTS);

        let mut id = self.last_listing_id().get();
        for payment in payments.iter() {
            require!(payment.token_nonce > 0, ERROR_ONLY_NFT);

            let listing = Listing {
                id,
                seller: caller.clone(),
                token_id: payment.token_identifier,
                token_nonce: payment.token_nonce,
                token_amount: payment.amount,
                listing_type,
                price: price.clone(),
                min_bid: min_bid.clone(),
                buyout_price: buyout_price.clone(),
                start_time,
                end_time,
            };
            self.listings(id).set(listing);
            self.seller_listings(caller.clone()).insert(id);
            id += 1;
        }
        self.last_listing_id().set(id);

        id
    }

    #[endpoint(removeListing)]
    fn remove_listing(&self, listing_id: u64) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        require!(!self.listings(listing_id).is_empty(), ERROR_LISTING_NOT_FOUND);

        let listing = self.listings(listing_id).get();
        let caller = self.blockchain().get_caller();
        require!(caller == listing.seller || caller == self.blockchain().get_owner_address(), ERROR_ONLY_LISTING_OWNER);

        self.send().direct_esdt(
            &caller,
            &listing.token_id,
            listing.token_nonce,
            &listing.token_amount,
        );
        self.do_remove_listing(&listing);
    }

    fn do_remove_listing(&self, listing: &Listing<Self::Api>) {
        if listing.listing_type == ListingType::Auction {
            let governance_token = self.governance_token().get();
            for bid_id in self.listing_bids(listing.id).iter() {
                let bid = self.bids(bid_id).get();
                self.send().direct_esdt(
                    &bid.bidder,
                    &governance_token,
                    0,
                    &bid.offer,
                );
                self.buyer_bids(&bid.bidder).swap_remove(&bid.id);
                self.bids(bid.id).clear();
            }
            self.listing_bids(listing.id).clear();
        }
        self.listings(listing.id).clear();
        self.seller_listings(listing.seller.clone()).swap_remove(&listing.id);
    }

    #[payable("*")]
    #[endpoint(buy)]
    fn buy(&self, listing_id: u64) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);

        let governance_token = self.governance_token().get();
        let payment = self.call_value().single_esdt();
        require!(payment.token_identifier == governance_token, ERROR_WRONG_PAYMENT);
        require!(!self.listings(listing_id).is_empty(), ERROR_LISTING_NOT_FOUND);

        let listing = self.listings(listing_id).get();
        let current_time = self.blockchain().get_block_timestamp();
        require!(!listing.has_expired(current_time), ERROR_LISTING_EXPIRED);
        require!(listing.has_started(current_time), ERROR_LISTING_NOT_STARTED);

        let expected_price = if listing.listing_type == ListingType::Auction {
            &listing.buyout_price
        } else {
            &listing.price
        };
        require!(&payment.amount == expected_price, ERROR_WRONG_PAYMENT);

        self.do_buy(&listing, payment.amount);
    }

    fn do_buy(&self, listing: &Listing<Self::Api>, price: BigUint) {
        let governance_token = self.governance_token().get();
        let caller = self.blockchain().get_caller();
        self.send().direct_esdt(
            &caller,
            &listing.token_id,
            listing.token_nonce,
            &listing.token_amount,
        );
        self.send().direct_esdt(
            &listing.seller,
            &governance_token,
            0,
            &price,
        );
        self.do_remove_listing(listing);
    }

    #[payable("*")]
    #[endpoint(addBid)]
    fn add_bid(&self, listing_id: u64) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);

        let payment = self.call_value().single_esdt();
        require!(payment.token_identifier == self.governance_token().get(), ERROR_WRONG_PAYMENT);
        require!(!self.listings(listing_id).is_empty(), ERROR_LISTING_NOT_FOUND);

        let listing = self.listings(listing_id).get();
        let current_time = self.blockchain().get_block_timestamp();
        require!(listing.listing_type == ListingType::Auction, ERROR_NOT_AUCTION);
        require!(!listing.has_expired(current_time), ERROR_LISTING_EXPIRED);
        require!(listing.has_started(current_time), ERROR_LISTING_NOT_STARTED);

        let (highest_bid_amount, _) = self.get_listing_last_bid(listing_id);
        require!(payment.amount > highest_bid_amount, ERROR_BID_TOO_LOW);

        let caller = self.blockchain().get_caller();
        let current_bid = self.get_buyer_bid_by_listing_id(&caller, listing_id);
        if current_bid.is_some() {
            self.remove_bid(current_bid.unwrap().id);
        }

        if payment.amount >= listing.buyout_price {
            let diff = &payment.amount - &listing.buyout_price;
            if diff > 0 {
                self.send().direct_esdt(
                    &caller,
                    &payment.token_identifier,
                    0,
                    &diff,
                );
            }
            self.do_buy(&listing, listing.buyout_price.clone());

            return
        }

        let bid_id = self.last_bid_id().get();
        let bid = Bid {
            id: bid_id,
            listing_id,
            bidder: caller.clone(),
            offer: payment.amount,
        };
        self.bids(bid_id).set(bid);
        self.buyer_bids(&caller).insert(bid_id);
        self.listing_bids(listing_id).insert(bid_id);
        self.last_bid_id().set(bid_id + 1);
    }

    #[endpoint(removeBid)]
    fn remove_bid(&self, bid_id: u64) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        require!(!self.bids(bid_id).is_empty(), ERROR_BID_NOT_FOUND);

        let bid = self.bids(bid_id).get();
        let caller = self.blockchain().get_caller();
        require!(bid.bidder == caller, ERROR_ONLY_BUYER);

        self.listing_bids(bid.listing_id).swap_remove(&bid.id);
        self.buyer_bids(&caller).swap_remove(&bid.id);
        self.bids(bid.id).clear();
        self.send().direct_esdt(
            &caller,
            &self.governance_token().get(),
            0,
            &bid.offer,
        );
    }

    #[endpoint(acceptBid)]
    fn accept_bid(&self, listing_id: u64) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        require!(!self.listings(listing_id).is_empty(), ERROR_LISTING_NOT_FOUND);

        let listing = self.listings(listing_id).get();
        require!(listing.listing_type == ListingType::Auction, ERROR_WRONG_BIDS);

        let current_time = self.blockchain().get_block_timestamp();
        require!(listing.has_expired(current_time), ERROR_LISTING_NOT_ENDED);

        let caller = self.blockchain().get_caller();
        require!(listing.seller == caller, ERROR_ONLY_LISTING_OWNER);

        let (highest_bid_amount, highest_bid) = self.get_listing_last_bid(listing_id);
        require!(highest_bid.is_some(), ERROR_BID_NOT_FOUND);

        self.send().direct_esdt(
            &highest_bid.unwrap().bidder,
            &listing.token_id,
            listing.token_nonce,
            &listing.token_amount,
        );
        self.send().direct_esdt(
            &caller,
            &self.governance_token().get(),
            0,
            &highest_bid_amount,
        );
        self.do_remove_listing(&listing);
    }

    // helpers
    fn check_whitelisted(&self, address: &ManagedAddress) {
        self.platform_contract_proxy()
            .contract(self.platform_sc().get())
            .check_whitelisted(address)
            .execute_on_dest_context::<()>();
    }
}
