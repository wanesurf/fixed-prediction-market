use coreum_wasm_sdk::types::cosmos::base::v1beta1::Coin;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Timestamp};

use crate::state::MarketOutcome;

#[cw_serde]
pub struct InstantiateMsg {
    pub buy_denom: String, // The denomination required to buy shares
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateMarket {
        id: String,
        options: Vec<String>,
        end_time: Timestamp,
        buy_token: String,
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
    #[returns(ShareResponse)]
    GetShares { market_id: String, user: Addr },
    #[returns(MarketStatsResponse)]
    GetMarketStats { market_id: String }, // Total value, number of bettors, odds
    #[returns(UserPotentialWinningsResponse)]
    GetUserPotentialWinnings { market_id: String, user: Addr }, // User's potential winnings
    #[returns(UserWinningsResponse)]
    GetUserWinnings { market_id: String, user: Addr }, // User's actual winnings
}

// We define a custom struct for each query response

#[cw_serde]
pub struct MarketResponse {
    pub id: String,
    pub options: Vec<String>,
    pub resolved: bool,
    pub outcome: MarketOutcome,
    pub end_time: Timestamp,
    pub total_value: Coin,
    pub num_bettors: u64,
}
#[cw_serde]
pub struct ShareResponse {
    pub user: Addr,
    pub option: String,
    pub amount: Coin,
}
#[cw_serde]
pub struct MarketStatsResponse {
    pub total_value: Coin,
    pub num_bettors: u64,
    pub odds_a: f64,
    pub odds_b: f64,
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
