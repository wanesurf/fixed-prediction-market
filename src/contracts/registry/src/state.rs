use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, StdError, Storage, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Config

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub oracle: Addr,
    pub buy_fee: Decimal,
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
}

#[cw_serde]
pub struct MarketPair {
    pub option: MarketOption,
    pub token: Coin,
}

#[cw_serde]
pub struct Share {
    pub user: Addr,
    pub pair: MarketPair, //represent the option and the token
    pub has_withdrawn: bool,
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
    pub pairs: Vec<MarketPair>, //represent the options and the tokens
    pub shares: Vec<Share>,     //represent the holder of the users
    pub resolved: bool,         //TODO replace by State with enum (MarketState)
    pub outcome: MarketOutcome,
    pub end_time: String,          // When the market ends
    pub total_value: Coin,         // Total value staked in the market
    pub num_bettors: u64,          // Number of unique bettors
    pub buy_token: String,         // Denom for the token used to buy shares
    pub banner_url: String,        // URL for the banner image
    pub description: String,       // Description of the market
    pub title: String,             // Title of the market
    pub end_time_string: String,   // End time of the market
    pub start_time_string: String, // Start time of the market
    pub resolution_source: String, // Source of the resolution --> Feed contract address
                                   // pub liquidity: String,         // Liquidity of the market
                                   //odds: <Vec<String>>,
}

/// Maps market_id -> MarketInfo (e.g., "truth_market_1" -> MarketInfo)
pub const MARKETS: Map<&str, MarketInfo> = Map::new("markets");
