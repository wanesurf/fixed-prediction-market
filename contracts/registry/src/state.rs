use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Timestamp};
use cw_storage_plus::{Item, Map};

/// Config

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub oracle: Addr,
    pub commission_rate: Decimal,
    //code IDs
    pub market_code_id: u64,
}
pub const CONFIG: Item<Config> = Item::new("config");

/// MARKET MANAGEMENT

//Note when we update a market we should the state of the registry as well
// Or should we call the registry to update the market? --> no
// Lets create a hook that will be called when the market is updated and update the registry state
// btw this is optional but it will be useful to have it

#[cw_serde]
pub enum MarketStatus {
    Pending,
    Active,
    Resolved,
    Cancelled,
    Expired,
}

#[cw_serde]
pub struct MarketOption {
    pub text: String, // The display text (e.g., "YES", "NO", "Trump", "Biden")
    pub associated_token_denom: String,
}

#[cw_serde]
pub enum MarketOutcome {
    Pending,
    Active,
    Unresolved,
    Resolved(MarketOption), // Store the winning option
    Cancelled,
    Expired, // Market has ended and no more bets can be made (transition state before resolved)
}

#[cw_serde]
pub struct MarketInfo {
    pub id: String,
    pub contract_address: Addr,
    pub pairs: Vec<MarketOption>, //represent the options and the tokens
    pub end_time: Timestamp,      // When the market ends
    pub start_time: Timestamp,    // When the market starts
    pub buy_token: String,        // Denom for the token used to buy shares
    pub banner_url: String,       // URL for the banner image
    pub description: String,      // Description of the market
    pub title: String,            // Title of the market
    pub resolution_source: String,
    pub oracle: Addr,
    pub commission_rate: Decimal,
    pub market_code_id: u64,
}

/// Maps market_id -> MarketInfo (e.g., "truth_market_1" -> MarketInfo)
pub const MARKETS: Map<&str, MarketInfo> = Map::new("markets");
