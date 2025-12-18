use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::error::ContractError;
use crate::state::{MarketInfo, MarketStatus, CONFIG, MARKETS};

pub fn execute_create_market(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: String,
    _options: Vec<String>,
    end_time: String,
    _buy_token: String,
    _banner_url: String,
    description: String,
    title: String,
    _end_time_string: String,
    _start_time_string: String,
    _resolution_source: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only admin can create markets through the registry
    if info.sender != config.admin {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "Unauthorized: Only admin can create markets",
        )));
    }

    // Check if the market ID already exists
    if MARKETS.has(deps.storage, &id) {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "Market ID already exists",
        )));
    }

    // For now, we'll store the market info directly in the registry
    // In the future, this could instantiate a separate market contract
    let market_info = MarketInfo {
        market_id: id.clone(),
        market_address: info.sender.clone(), // Registry holds the market for now
        status: MarketStatus::Active,
        title: title.clone(),
        description: description.clone(),
        end_time: end_time.clone(),
        created_by: info.sender.clone(),
    };

    MARKETS.save(deps.storage, &id, &market_info)?;

    Ok(Response::new()
        .add_attribute("action", "create_market")
        .add_attribute("market_id", id)
        .add_attribute("title", title)
        .add_attribute("created_by", info.sender))
}