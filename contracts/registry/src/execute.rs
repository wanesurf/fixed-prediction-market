use cosmwasm_std::{
    to_json_binary, Addr, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StdError,
    SubMsg, Timestamp, Uint128, WasmMsg,
};

use crate::error::ContractError;
use crate::state::{MarketInfo, MarketOption, MarketStatus, CONFIG, MARKETS};

use market::msg::{InstantiateMsg as MarketInstantiateMsg, MarketType};

use utils::{address::derive_address2, hashing::hash_data, validation::validate_funds};

pub fn execute_create_market(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
    options: Vec<String>,
    buy_token: String,
    banner_url: String,
    description: String,
    title: String,
    start_time: Timestamp,
    end_time: Timestamp,
    resolution_source: String,
    asset_to_track: String,
    market_type: MarketType,
    target_price: Decimal,
    oracle: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let admin = config.admin.clone();

    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only the admin can create markets",
        )));
    }

    let payment: cosmwasm_std::Uint128 =
        validate_funds(&info, "ucore", Some(Uint128::from(20_000_000u128)))
            .map_err(|e| ContractError::Std(StdError::generic_err(e.to_string())))?;

    // Check if the market ID already exists
    if MARKETS.has(deps.storage, &id) {
        return Err(ContractError::Std(StdError::generic_err(
            "Market ID already exists",
        )));
    }

    if options.len() != 2 {
        return Err(ContractError::Std(StdError::generic_err(
            "Markets must have exactly two options",
        )));
    }

    // create denom for the options (we redo the same thing in the market contract)
    let subunit_token_a = format!(
        "truth{}_{}",
        options[0].to_lowercase().replace(" ", "_"),
        id.to_lowercase().replace(" ", "_")
    );

    //TODO here we need to use the futur contract address (market)
    let denom_token_a: String = format!("{}-{}", subunit_token_a, "");
    let subunit_token_b = format!(
        "truth{}_{}",
        options[1].to_lowercase().replace(" ", "_"),
        id.to_lowercase().replace(" ", "_")
    );
    //TODO here we need to use the futur contract address (market)
    let denom_token_b: String = format!("{}-{}", subunit_token_b, "");

    let market_instantiate_msg = MarketInstantiateMsg {
        id: id.clone(),
        admin: config.admin.clone(),
        end_time: end_time.clone(),
        buy_token: buy_token.clone(),
        banner_url: banner_url.clone(),
        description: description.clone(),
        title: title.clone(),
        start_time: start_time.clone(),
        resolution_source: resolution_source.clone(),
        asset_to_track: asset_to_track.clone(),
        market_type: market_type.clone(),
        target_price: target_price.clone(),
        commission_rate: config.commission_rate,
        oracle: oracle.clone(),
    };

    let registry_canonical_addr = deps.api.addr_canonicalize(env.contract.address.as_str())?;
    // derive the market address from the code id
    let market_code_info = deps.querier.query_wasm_code_info(config.market_code_id)?;
    let market_hash = hash_data(vec![&id]);
    let market_canonical_addr: cosmwasm_std::CanonicalAddr = derive_address2(
        registry_canonical_addr.clone(),
        market_hash.as_slice(),
        &market_code_info.checksum.to_hex(),
    )?;
    let market_addr = deps.api.addr_humanize(&market_canonical_addr)?;

    //save the market info in the registry with the market address
    let market_info = MarketInfo {
        id: id.clone(),
        contract_address: (market_addr.clone()),
        pairs: vec![
            MarketOption {
                text: options[0].clone(),
                associated_token_denom: denom_token_a.clone(),
            },
            MarketOption {
                text: options[1].clone(),
                associated_token_denom: denom_token_b.clone(),
            },
        ],
        end_time: end_time.clone(),
        start_time: start_time.clone(),
        buy_token: buy_token.clone(),
        banner_url: banner_url.clone(),
        description: description.clone(),
        title: title.clone(),
        resolution_source: resolution_source.clone(),
        oracle: oracle.clone(),
        commission_rate: config.commission_rate,
        market_code_id: config.market_code_id,
    };

    MARKETS.save(deps.storage, &id, &market_info)?;

    Ok(Response::new()
        .add_attribute("action", "create_market")
        .add_attribute("market_id", id)
        .add_attribute("title", title)
        .add_attribute("created_by", info.sender)
        .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Instantiate2 {
            admin: Some(admin.clone().to_string()),
            code_id: config.market_code_id,
            label: "cruise_control_prediction_market".to_string(),
            msg: to_json_binary(&market_instantiate_msg)?,
            //we need to pay for the two FTs to be created (20 COREUM)
            funds: vec![Coin {
                denom: "ucore".to_string(),
                amount: payment.clone(),
            }],
            salt: market_hash.as_slice().into(),
        }))))
}
