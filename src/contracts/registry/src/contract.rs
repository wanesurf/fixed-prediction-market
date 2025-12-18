#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_json_binary};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG};
use crate::execute;
use crate::query;

/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:registry";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        admin: info.sender.clone(),
        oracle: msg.oracle,
        buy_fee: msg.buy_fee,
        market_code_id: msg.market_code_id,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateMarket {
            id,
            options,
            end_time,
            buy_token,
            banner_url,
            description,
            title,
            end_time_string,
            start_time_string,
            resolution_source,
        } => execute::execute_create_market(
            deps,
            env,
            info,
            id,
            options,
            end_time,
            buy_token,
            banner_url,
            description,
            title,
            end_time_string,
            start_time_string,
            resolution_source,
        ),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_json_binary(&query::query_config(deps)?),
        QueryMsg::Market { market_id } => to_json_binary(&query::query_market(deps, market_id)?),
        QueryMsg::ListMarkets {} => to_json_binary(&query::query_list_markets(deps)?),
    }
}

#[cfg(test)]
mod tests {}