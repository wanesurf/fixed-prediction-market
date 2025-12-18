use coreum_wasm_sdk::types::{
    coreum::asset::ft::v1::QueryBalanceResponse, cosmos::base::v1beta1::Coin,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal};

use crate::state::{MarketOutcome, MarketPair};

#[cw_serde]
pub struct InstantiateMsg {
    pub buy_denom: String, // The denomination required to buy shares
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateMarket {
        id: String,
        options: Vec<String>, //outcomes options
        end_time: String,
        buy_token: String,
        banner_url: String,
        description: String,
        title: String,
        end_time_string: String,
        start_time_string: String,
        resolution_source: String,
    },
    BuyShare {
        market_id: String,
        option: String,
        amount: Coin,
    },
    Resolve {
        market_id: String,
        winning_option: String,
    },
    Withdraw {
        market_id: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(MarketResponse)]
    GetMarket { id: String },
    #[returns(AllSharesResponse)]
    GetShares { market_id: String, user: Addr },
    #[returns(MarketStatsResponse)]
    GetMarketStats { market_id: String }, // Total value, number of bettors, odds
    #[returns(UserPotentialWinningsResponse)]
    GetUserPotentialWinnings { market_id: String, user: Addr }, // User's potential winnings
    #[returns(UserWinningsResponse)]
    GetUserWinnings { market_id: String, user: Addr }, // User's actual winnings
    #[returns(QueryBalanceResponse)]
    GetUserBalance { user: String, denom: String }, // User's balance
    #[returns(AllSharesResponse)]
    GetAllShares { market_id: String }, // All shares
    //Get total stakes: Amount of tokens in the market PER option
    #[returns(TotalValueResponse)]
    GetTotalValue { market_id: String }, // Total stakes
    #[returns(TotalSharesPerOptionResponse)]
    GetTotalSharesPerOption { market_id: String }, // Total shares per option
    #[returns(OddsResponse)] //TOD):rename!
    GetOdds { market_id: String }, // Total shares per option
}

// We define a custom struct for each query response

#[cw_serde]
pub struct MarketResponse {
    pub id: String,
    pub options: Vec<String>,
    pub resolved: bool,
    pub outcome: MarketOutcome,
    pub end_time: String,
    pub total_value: Coin,
    pub num_bettors: u64,
    pub token_a: Coin,
    pub token_b: Coin,
    pub buy_token: String,
    pub banner_url: String,
    pub description: String,
    pub title: String,
    pub end_time_string: String,
    pub start_time_string: String,
    pub resolution_source: String,
    // pub liquidity: String
}

#[cw_serde]
pub struct OddsResponse {
    pub odds_a: Decimal,
    pub odds_b: Decimal,
}

#[cw_serde]
pub struct TotalValueResponse {
    pub total_value: Coin,
}

#[cw_serde]
pub struct TotalSharesPerOptionResponse {
    pub pair_a: MarketPair,
    pub pair_b: MarketPair,
}

#[cw_serde]
pub struct ShareResponse {
    pub user: Addr,
    pub option: String,
    pub amount: Coin,
    pub has_withdrawn: bool, //
}
#[cw_serde]
pub struct AllSharesResponse {
    pub shares: Vec<ShareResponse>,
}
#[cw_serde]
pub struct MarketStatsResponse {
    pub total_value: Coin,
    pub num_bettors: u64,
    pub odds_a: Decimal,
    pub odds_b: Decimal,
}
#[cw_serde]
pub struct UserPotentialWinningsResponse {
    pub potential_win_a: Coin,
    pub potential_win_b: Coin,
}
#[cw_serde]
pub struct UserWinningsResponse {
    pub winnings: Coin,
}
