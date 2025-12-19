use cosmwasm_std::{Deps, StdResult};

use crate::state::{Config, MarketInfo, CONFIG, MARKETS};

pub fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

pub fn query_market(deps: Deps, market_id: String) -> StdResult<MarketInfo> {
    MARKETS.load(deps.storage, &market_id)
}

pub fn query_list_markets(deps: Deps) -> StdResult<Vec<MarketInfo>> {
    let markets: StdResult<Vec<_>> = MARKETS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(_, market_info)| market_info))
        .collect();
    markets
}