use crate::state::{Config, MarketInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Timestamp};

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct InstantiateMsg {
    pub oracle: Addr,
    pub commission_rate: Decimal,
    pub market_code_id: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateMarket {
        id: String,
        options: Vec<String>, //outcomes options
        start_time: Timestamp,
        end_time: Timestamp,
        buy_token: String,
        banner_url: String,
        description: String,
        title: String,
        resolution_source: String,
        oracle: Addr,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    GetConfig {},
    #[returns(MarketInfo)]
    Market { market_id: String },
    #[returns(Vec<MarketInfo>)]
    ListMarkets {},
}

// Market contract messages for instantiation and execution
#[cw_serde]
pub struct MarketInstantiateMsg {
    pub buy_denom: String,
}

#[cw_serde]
pub enum MarketExecuteMsg {
    CreateMarket {
        id: String,
        options: Vec<String>,
        end_time: String,
        buy_token: String,
        banner_url: String,
        description: String,
        title: String,
        end_time_string: String,
        start_time_string: String,
        resolution_source: String,
    },
}
