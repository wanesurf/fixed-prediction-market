use coreum_wasm_sdk::types::{
    coreum::asset::ft::v1::QueryBalanceResponse, cosmos::base::v1beta1::Coin,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Timestamp};

use crate::state::{MarketOption, MarketStatus};

#[cw_serde]
pub struct InstantiateMsg {
    pub id: String,
    pub options: Vec<String>, //outcomes options
    pub start_time: Timestamp,
    pub end_time: Timestamp,
    pub commission_rate: Decimal,
    pub buy_token: String,
    pub banner_url: String,
    pub description: String,
    pub title: String,
    pub resolution_source: String,
    pub oracle: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    BuyShare {
        market_id: String,
        option: String,
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
    pub status: MarketStatus,
    pub total_value: Coin,
    pub num_bettors: u64,
    pub token_a: Coin,
    pub token_b: Coin,
    pub buy_token: String,
    pub banner_url: String,
    pub description: String,
    pub title: String,
    pub end_time: Timestamp,
    pub start_time: Timestamp,
    pub resolution_source: String,
    // pub liquidity: String
}

#[cw_serde]
pub struct OptionOdds {
    pub option: String,
    pub odds: Decimal,
}

#[cw_serde]
pub struct OddsResponse {
    pub odds: Vec<OptionOdds>,
}

#[cw_serde]
pub struct TotalValueResponse {
    pub total_value: Coin,
}

#[cw_serde]
pub struct TotalSharesPerOptionResponse {
    pub option_a: MarketOption,
    pub amount_a: Coin,
    pub option_b: MarketOption,
    pub amount_b: Coin,
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
