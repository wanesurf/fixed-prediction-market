use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, Decimal, Deps, DepsMut, Env, Event, MessageInfo, Response, StdError, StdResult, to_json_binary
};
use cw2::set_contract_version;

/// TODO: Price per share at each buy and sell
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{
    Config, MarketOption, MarketState, MarketStatus, Share, CONFIG, MARKET_STATE, SHARES,
};
use cosmwasm_std::{CosmosMsg, Uint128};

//Coreum related imports
use coreum_wasm_sdk::types::coreum::asset::ft::v1::MsgMint;
use coreum_wasm_sdk::types::coreum::asset::ft::v1::{MsgBurn, MsgIssue};

use coreum_wasm_sdk::types::cosmos::bank::v1beta1::MsgSend;
use coreum_wasm_sdk::types::cosmos::base::v1beta1::Coin;

use cw_utils::must_pay;

use clp_feed_interface::ClpFeedQuerier;


// Contract name and version for migration
const CONTRACT_NAME: &str = "crates.io:cruise-control-prediction-market";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    //NOTES: Each market will cost at least 20 COREUM to create (2 FT tokens creates)

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Get the options for this market type
    let options = msg.market_type.get_options();

    // Get initial price from clp_feed contract

    let oracle = ClpFeedQuerier::new(&deps.querier, msg.oracle.clone());
    let initial_price = oracle.query_price(msg.asset_to_track.clone())?;
    if initial_price.price.is_none() {
        return Err(ContractError::Std(StdError::generic_err(
            "Failed to get initial price from clp_feed contract",
        )));
    }



    let subunit_token_a = format!(
        "truth{}_{}",
        options[0].to_lowercase().replace(" ", "_"),
        msg.id.to_lowercase().replace(" ", "_")
    );

    let symbol_token_a = format!(
        "TM{}{}", // TM prefix for "Truth Markets"
        options[0].replace(" ", ""),
        msg.id.replace(" ", "")
    );

    // Issue two new smart tokens for the market options
    let issue_token_a = MsgIssue {
        issuer: env.contract.address.to_string(),
        symbol: symbol_token_a.clone(),
        subunit: subunit_token_a.clone(),
        precision: 6,
        initial_amount: "0".to_string(),
        description: format!("Token for {} in market {}", options[0], msg.id),
        //Minting & Burning is enabled
        features: vec![0 as i32, 1 as i32],
        burn_rate: "0".to_string(),
        send_commission_rate: "0".to_string(),
        uri: "https://app.cruise-control.xyz/dashboard".to_string(),
        uri_hash: "".to_string(),
        extension_settings: None,
        dex_settings: None,
    };

    let denom_token_a: String = format!("{}-{}", subunit_token_a, env.contract.address);

    let subunit_token_b = format!(
        "truth{}_{}",
        options[1].to_lowercase().replace(" ", "_"),
        msg.id.to_lowercase().replace(" ", "_")
    );

    let symbol_token_b = format!(
        "TM{}{}", // TM prefix for "Truth Markets"
        options[1].replace(" ", ""),
        msg.id.replace(" ", "")
    );

    let issue_token_b = MsgIssue {
        issuer: env.contract.address.to_string(),
        symbol: symbol_token_b.clone(),
        subunit: subunit_token_b.clone(),
        precision: 6,
        initial_amount: "0".to_string(),
        description: format!("Token for {} in market {}", options[1], msg.id),
        features: vec![0 as i32, 1 as i32],
        burn_rate: "0".to_string(),
        send_commission_rate: "0".to_string(),
        uri: "https://app.cruise-control.xyz".to_string(),
        uri_hash: "".to_string(),
        extension_settings: None,
        dex_settings: None,
    };

    let denom_token_b = format!("{}-{}", subunit_token_b, env.contract.address);

    // Create MarketOption structs with associated token denoms
    let option_a = MarketOption {
        text: options[0].clone(),
        associated_token_denom: denom_token_a.clone(),
    };

    let option_b = MarketOption {
        text: options[1].clone(),
        associated_token_denom: denom_token_b.clone(),
    };

    let market_state = MarketState {
        status: MarketStatus::Pending,
        num_bettors: 0,
        total_value: Coin {
            denom: msg.buy_token.clone(),
            amount: "0".to_string(),
        },
        total_stake_option_a: Uint128::zero(),
        total_stake_option_b: Uint128::zero(),
        volume: Uint128::zero(),
    };

    let market_config = Config {
        id: msg.id.clone(),
        admin: msg.admin.clone(),
        commission_rate: msg.commission_rate.clone(),
        pairs: vec![option_a, option_b],
        start_time: msg.start_time.clone(),
        end_time: msg.end_time.clone(),
        buy_token: msg.buy_token.clone(),
        banner_url: msg.banner_url.clone(),
        description: msg.description.clone(),
        title: msg.title.clone(),
        oracle: msg.oracle.clone(),
        resolution_source: msg.resolution_source.clone(),
        asset_to_track: msg.asset_to_track.clone(),
        market_type: msg.market_type.clone(),
        target_price: msg.target_price.clone(),
        //TODO: check this
        initial_price: Decimal::from_str(&initial_price.price.unwrap().price).unwrap(),
    };

    MARKET_STATE.save(deps.storage, &market_state)?;
    CONFIG.save(deps.storage, &market_config)?;

    Ok(Response::new()
        .add_event(
            Event::new("cc_prediction_market_create_market")
                .add_attribute("market_id", msg.id)
                .add_attribute("admin", msg.admin)
                .add_attribute("commission_rate", msg.commission_rate.to_string())
                .add_attribute("buy_token", msg.buy_token)
                .add_attribute("banner_url", msg.banner_url)
                .add_attribute("description", msg.description)
                .add_attribute("title", msg.title)
                .add_attribute("start_time", msg.start_time.to_string())
                .add_attribute("end_time", msg.end_time.to_string())
                .add_attribute("resolution_source", msg.resolution_source)
                .add_attribute("oracle", msg.oracle)
                .add_attribute("asset_to_track", msg.asset_to_track)
                .add_attribute("market_type", msg.market_type.to_string())
                .add_attribute("target_price", msg.target_price.to_string())
                .add_attribute("initial_price", market_config.initial_price.to_string())
                .add_attribute(
                    "initial_odds",
                    cosmwasm_std::to_json_string(&msg.market_type.create_option_odds(Decimal::zero(), Decimal::zero()))
                    .unwrap_or_else(|_| "[]".to_string()),
                )
        )
        .add_message(CosmosMsg::Any(issue_token_a.to_any()))
        .add_message(CosmosMsg::Any(issue_token_b.to_any())))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::BuyShare { market_id, option } => buy_share(deps, env, info, market_id, option),
        ExecuteMsg::Resolve {
            market_id,
        } => resolve(deps, env, info, market_id),
        ExecuteMsg::Withdraw { market_id } => withdraw(deps, env, info, market_id),
        ExecuteMsg::SellShare { option } => sell_share(deps, env, info, option),
    }
}

pub fn sell_share(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    option: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut market_state = MARKET_STATE.load(deps.storage)?;

    // Check if market is still active (can't sell after resolved)
    if matches!(market_state.status, MarketStatus::Resolved(_)) {
        return Err(ContractError::Std(StdError::generic_err(
            "Cannot sell shares after market is resolved",
        )));
    }

    // Find the matching market option
    let market_option = config
        .pairs
        .iter()
        .find(|p| p.text == option)
        .cloned()
        .ok_or_else(|| StdError::generic_err("Invalid option"))?;

    let associated_token_denom = market_option.associated_token_denom.clone();

    // Amount the user wants to sell
    let amount_sent = must_pay(&info, &associated_token_denom)?;

    // Calculate time-based tax
    let tax_rate = market_state.calculate_time_based_tax(&config, env.block.time);
    let amount_after_tax =
        market_state.calculate_sell_amount_with_tax(&config, amount_sent, env.block.time);
    let tax_amount = amount_sent - amount_after_tax;

    // Calculate commission on the amount after tax using BPS (basis points)
    let commission_amount = amount_after_tax * config.commission_rate / Uint128::from(10000u128);
    let final_amount = amount_after_tax - commission_amount;

    // Update share using Map - O(1) operation
    SHARES.update(
        deps.storage,
        (&info.sender, &market_option.text),
        |existing| -> StdResult<Share> {
            match existing {
                Some(mut share) => {
                    if share.amount < amount_sent {
                        return Err(StdError::generic_err("Insufficient shares to sell"));
                    }
                    share.amount -= amount_sent;
                    Ok(share)
                }
                None => Err(StdError::generic_err("No shares found for user")),
            }
        },
    )?;

    // Update aggregate totals - reduce by final amount (user's effective stake decrease)
    if market_option.text == config.pairs[0].text {
        market_state.total_stake_option_a -= final_amount;
    } else {
        market_state.total_stake_option_b -= final_amount;
    }

    // Update total value - only reduce by the amount returned to user
    // The tax and commission amounts stay in the pot, benefiting remaining participants
    let new_total_value =
        Uint128::from_str(&market_state.total_value.amount).unwrap() - final_amount;

    market_state.total_value.amount = new_total_value.to_string();

    // Update volume
    market_state.volume += amount_sent;

    // Save the updated market state
    MARKET_STATE.save(deps.storage, &market_state)?;

    // Burn the tokens
    let burn_msg = MsgBurn {
        sender: env.contract.address.to_string(),
        coin: Some(Coin {
            denom: market_option.associated_token_denom.clone(),
            amount: amount_sent.to_string(),
        }),
    };

    // Send back only the final amount to the user
    // The tax and commission amounts effectively stay in the market pot, benefiting remaining participants
    let mut messages = vec![CosmosMsg::Any(burn_msg.to_any())];

    if final_amount > Uint128::zero() {
        let return_msg = MsgSend {
            from_address: env.contract.address.to_string(),
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                denom: config.buy_token.clone(),
                amount: final_amount.to_string(),
            }],
        };

        //send comission to admin
        let commission_to_admin_msg = MsgSend {
            from_address: env.contract.address.to_string(),
            to_address: config.admin.to_string(),
            amount: vec![Coin {
                denom: config.buy_token.clone(),
                amount: commission_amount.to_string(),
            }],
        };
        
        messages.push(CosmosMsg::Any(return_msg.to_any()));
        messages.push(CosmosMsg::Any(commission_to_admin_msg.to_any()))
    }

    let config = CONFIG.load(deps.storage)?;

    let response = Response::new();

    Ok(response
        .add_event(
            Event::new("cc_prediction_market_sell_share")
                .add_attribute("market_id", config.clone().id)
                .add_attribute("option", market_option.text)
                .add_attribute("tokens_sent", amount_sent.to_string())
                .add_attribute("amount_after_tax", amount_after_tax.to_string())
                .add_attribute("tax_amount", tax_amount.to_string())
                .add_attribute("tax_rate", tax_rate.to_string())
                .add_attribute("commission_amount", commission_amount.to_string())
                .add_attribute("final_amount", final_amount.to_string())
                .add_attribute("user", info.sender)
                .add_attribute("total_value", new_total_value.to_string())
                .add_attribute("total_volume", market_state.volume.to_string())
                .add_attribute(
                    "odds",
                    cosmwasm_std::to_json_string(&market_state.create_type_safe_odds(&config))
                    .unwrap_or_else(|_| "[]".to_string()),
                ),
        )
        .add_messages(messages))
}

pub fn buy_share(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _market_id: String,
    option: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut market_state = MARKET_STATE.load(deps.storage)?;

    let payment: Uint128 = must_pay(&info, &config.buy_token)?;

    // Calculate commission using BPS (basis points)
    let commission_amount = payment * config.commission_rate / Uint128::from(10000u128);
    let net_payment = payment - commission_amount;

    // Check if the market is already resolved
    if matches!(market_state.status, MarketStatus::Resolved(_)) {
        return Err(ContractError::Std(StdError::generic_err(
            "Market is already resolved",
        )));
    }

    // Find the matching market option
    let market_option = config
        .pairs
        .iter()
        .find(|p| p.text == option)
        .cloned()
        .ok_or_else(|| StdError::generic_err("Invalid option"))?;

    // Check if this is a new bettor (no existing shares for either option)
    let has_any_shares = SHARES
        .may_load(deps.storage, (&info.sender, &config.pairs[0].text))?
        .is_some()
        || SHARES
            .may_load(deps.storage, (&info.sender, &config.pairs[1].text))?
            .is_some();

    if !has_any_shares {
        market_state.num_bettors += 1;
    }

    // Update or create share using Map - O(1) operation
    SHARES.update(
        deps.storage,
        (&info.sender, &market_option.text),
        |existing| -> StdResult<Share> {
            match existing {
                Some(mut share) => {
                    share.amount += net_payment;
                    Ok(share)
                }
                None => Ok(Share {
                    amount: net_payment,
                    has_withdrawn: false,
                }),
            }
        },
    )?;

    // Update aggregate totals
    if market_option.text == config.pairs[0].text {
        market_state.total_stake_option_a += net_payment;
    } else {
        market_state.total_stake_option_b += net_payment;
    }

    // Update volume
    market_state.volume += payment;

    // Update total value
    let new_total_value = Uint128::from_str(&market_state.total_value.amount).unwrap() + net_payment;
    market_state.total_value.amount = new_total_value.to_string();

    // Save the updated market state
    MARKET_STATE.save(deps.storage, &market_state)?;

    // Mint tokens to user
    let mint_msg = MsgMint {
        sender: env.contract.address.to_string(),
        coin: Some(Coin {
            denom: market_option.associated_token_denom.clone(),
            amount: net_payment.to_string(),
        }),
        recipient: info.sender.to_string(),
    };

       //send comission to admin
    let commission_to_admin_msg = MsgSend {
        from_address: env.contract.address.to_string(),
        to_address: config.admin.to_string(),
        amount: vec![Coin {
            denom: config.buy_token.clone(),
            amount: commission_amount.to_string(),
        }],
    };


    let response = Response::new();

    // Use type-safe odds creation
    let odds = market_state.create_type_safe_odds(&config);

    let total_value = Uint128::from_str(&market_state.total_value.amount).unwrap();

    Ok(response
        .add_event(
            Event::new("cc_prediction_market_buy_share")
                .add_attribute("market_id", config.id)
                .add_attribute("option", market_option.text)
                .add_attribute("amount", payment.to_string())
                .add_attribute("net_amount", net_payment.to_string())
                .add_attribute("commission_amount", commission_amount.to_string())
                .add_attribute("user", info.sender.to_string())
                .add_attribute("total_value", total_value.to_string())
                .add_attribute("total_volume", market_state.volume.to_string())
                .add_attribute(
                    "odds",
                    cosmwasm_std::to_json_string(&odds).unwrap_or_else(|_| "[]".to_string()),
                ),
        )
        .add_message(CosmosMsg::Any(commission_to_admin_msg.to_any()))
        .add_message(CosmosMsg::Any(mint_msg.to_any())))
}

pub fn resolve(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _market_id: String,
) -> Result<Response, ContractError> {

    //TODO: check how can we get the price at resolve time!
    //Right now we get the latest price wihcih might be different then the price at end_time!
    //Etheir a relayer call this function at the right time or maybe clp_feed can keep history
    let config = CONFIG.load(deps.storage)?;
    let mut market_state = MARKET_STATE.load(deps.storage)?;

    // Ensure only the admin can resolve the market --> The relayer
    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only the admin can resolve markets",
        )));
    }

    // Check if the market has ended
    if env.block.time < config.end_time {
        return Err(ContractError::Std(StdError::generic_err(
            "Market has not ended yet",
        )));
    }

    // Check if the market is already resolved
    if matches!(market_state.status, MarketStatus::Resolved(_)) {
        return Err(ContractError::Std(StdError::generic_err(
            "Market is already resolved",
        )));
    }

    // Get current price from oracle
    let oracle = ClpFeedQuerier::new(&deps.querier, config.oracle.clone());
    let current_price_response = oracle.query_price(config.asset_to_track.clone())?;

    let current_price = match current_price_response.price {
        Some(price_info) => Decimal::from_str(&price_info.price)
            .map_err(|_| StdError::generic_err("Invalid price format from oracle"))?,
        None => return Err(ContractError::Std(StdError::generic_err(
            "No price available from oracle"
        ))),
    };

    // Determine winning option based on market type and price comparison
    let winning_option_text = config.market_type.determine_winner(current_price, config.target_price);

    let winning_option_obj = config
        .pairs
        .iter()
        .find(|p| p.text == winning_option_text)
        .cloned()
        .ok_or_else(|| StdError::generic_err(
            format!("Could not find option")
        ))?;

    // Calculate type-safe final odds before updating the market state
    let final_odds = market_state.create_type_safe_odds(&config);

    // Update the market status with the winning option
    market_state.status = MarketStatus::Resolved(winning_option_obj);

    // Save the updated market state
    MARKET_STATE.save(deps.storage, &market_state)?;

    Ok(Response::new().add_event(
        Event::new("cc_prediction_market_resolve")
            .add_attribute("market_id", config.id)
            .add_attribute("winning_option", winning_option_text)
            .add_attribute("current_price", current_price.to_string())
            .add_attribute("target_price", config.target_price.to_string())
            .add_attribute("initial_price", config.initial_price.to_string())
            .add_attribute("user", info.sender.to_string())
            .add_attribute("total_value", market_state.total_value.amount.to_string())
            .add_attribute(
                "final_odds",
                cosmwasm_std::to_json_string(&final_odds).unwrap_or_else(|_| "[]".to_string()),
            ),
    ))
}

pub fn withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    market_id: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let market_state = MARKET_STATE.load(deps.storage)?;

    // Get winning option (also checks if market is resolved)
    let winning_option = match &market_state.status {
        MarketStatus::Resolved(option) => option,
        _ => {
            return Err(ContractError::Std(StdError::generic_err(
                "Market is not resolved yet",
            )))
        }
    };

    // Check that the user sent the winning token denom to withdraw their winnings
    let _amount: Uint128 = must_pay(&info, &winning_option.associated_token_denom)?;

    // Load and check user's winning share using Map - O(1) operation
    let share = SHARES
        .may_load(deps.storage, (&info.sender, &winning_option.text))?
        .ok_or_else(|| StdError::generic_err("No winning shares found for user"))?;

    if share.has_withdrawn {
        return Err(ContractError::Std(StdError::generic_err(
            "User has already withdrawn their winnings",
        )));
    }

    // Mark share as withdrawn
    SHARES.update(
        deps.storage,
        (&info.sender, &winning_option.text),
        |existing| -> StdResult<Share> {
            match existing {
                Some(mut s) => {
                    s.has_withdrawn = true;
                    Ok(s)
                }
                None => Err(StdError::generic_err("No shares found")),
            }
        },
    )?;

    let total_winnings = market_state.calculate_winnings(deps.storage, &info.sender, &config)?;

    // Create bank transfer message
    let transfer_msg = MsgSend {
        from_address: env.contract.address.to_string(),
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            amount: total_winnings.amount.clone(),
            denom: total_winnings.denom,
        }],
    };

    let response = Response::new();

    Ok(response
        .add_event(
            Event::new("cc_prediction_market_withdraw")
                .add_attribute("market_id", market_id)
                .add_attribute("user", info.sender.to_string())
                .add_attribute("winning_option", winning_option.text.clone())
                .add_attribute("total_winnings", total_winnings.amount.to_string()),
        )
        .add_message(CosmosMsg::Any(transfer_msg.to_any())))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetMarket { id } => to_json_binary(&query::query_market(deps, id)?),
        QueryMsg::GetShares { market_id, user } => {
            to_json_binary(&query::query_shares(deps, market_id, user)?)
        }
        QueryMsg::GetMarketStats { market_id } => {
            to_json_binary(&query::query_market_stats(deps, market_id)?)
        }
        QueryMsg::GetUserWinnings { market_id, user } => {
            to_json_binary(&query::query_user_winnings(deps, market_id, user)?)
        }
        QueryMsg::GetUserPotentialWinnings { market_id, user } => to_json_binary(
            &query::query_user_potential_winnings(deps, market_id, user)?,
        ),
        QueryMsg::GetUserBalance { user, denom } => {
            to_json_binary(&query::query_balance(deps, user, denom)?)
        }
        QueryMsg::GetAllShares { market_id } => {
            to_json_binary(&query::query_all_shares(deps, market_id)?)
        }
        QueryMsg::GetTotalValue { market_id: _ } => {
            to_json_binary(&query::query_total_value(deps)?)
        }
        QueryMsg::GetTotalSharesPerOption { market_id } => {
            to_json_binary(&query::query_total_shares_per_option(deps, market_id)?)
        }
        QueryMsg::GetOdds { market_id: _ } => to_json_binary(&query::query_odds(deps)?),
        QueryMsg::GetTaxRate {} => to_json_binary(&query::query_tax_rate(deps, _env)?),
        QueryMsg::SimulateSell { option, amount } => {
            to_json_binary(&query::query_simulate_sell(deps, _env, option, amount)?)
        }
    }
}
pub mod query {
    use coreum_wasm_sdk::types::coreum::asset::ft::v1::{
        QueryBalanceRequest, QueryBalanceResponse,
    };
    use cosmwasm_std::Addr;

    use crate::msg::{
        AllSharesResponse, MarketResponse, MarketStatsResponse, OddsResponse, ShareResponse,
        SimulateSellResponse, TaxRateResponse, TotalSharesPerOptionResponse, TotalValueResponse,
        UserPotentialWinningsResponse, UserWinningsResponse,
    };

    use super::*;

    pub fn query_odds(deps: Deps) -> StdResult<OddsResponse> {
        let market_state = MARKET_STATE.load(deps.storage)?;
        let config = CONFIG.load(deps.storage)?;

        Ok(OddsResponse {
            odds: market_state.create_type_safe_odds(&config),
        })
    }

    pub fn query_total_value(deps: Deps) -> StdResult<TotalValueResponse> {
        let market_state = MARKET_STATE.load(deps.storage)?;

        Ok(TotalValueResponse {
            total_value: market_state.total_value,
        })
    }

    pub fn query_total_shares_per_option(
        deps: Deps,
        _market_id: String,
    ) -> StdResult<TotalSharesPerOptionResponse> {
        let market_state = MARKET_STATE.load(deps.storage)?;
        let config = CONFIG.load(deps.storage)?;
        let (total_a, total_b) = market_state.total_stakes(&config);

        let options = vec![
            crate::msg::OptionShares {
                option: config.pairs[0].text.clone(),
                token_denom: config.pairs[0].associated_token_denom.clone(),
                total_staked: Coin {
                    denom: config.buy_token.clone(),
                    amount: total_a.to_string(),
                },
            },
            crate::msg::OptionShares {
                option: config.pairs[1].text.clone(),
                token_denom: config.pairs[1].associated_token_denom.clone(),
                total_staked: Coin {
                    denom: config.buy_token.clone(),
                    amount: total_b.to_string(),
                },
            },
        ];

        Ok(TotalSharesPerOptionResponse {
            options,
            // Keep legacy fields for backward compatibility
            option_a: config.pairs[0].clone(),
            amount_a: Coin {
                denom: config.buy_token.clone(),
                amount: total_a.to_string(),
            },
            option_b: config.pairs[1].clone(),
            amount_b: Coin {
                denom: config.buy_token.clone(),
                amount: total_b.to_string(),
            },
        })
    }

    pub fn query_all_shares(deps: Deps, _market_id: String) -> StdResult<AllSharesResponse> {
        let config = CONFIG.load(deps.storage)?;

        // Iterate over all shares in the Map
        let shares: Vec<ShareResponse> = SHARES
            .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .map(|item| {
                let ((user, option_text), share) = item?;
                Ok(ShareResponse {
                    user,
                    option: option_text,
                    amount: Coin {
                        denom: config.buy_token.clone(),
                        amount: share.amount.to_string(),
                    },
                    has_withdrawn: share.has_withdrawn,
                })
            })
            .collect::<StdResult<Vec<_>>>()?;

        Ok(AllSharesResponse { shares })
    }

    pub fn query_market(deps: Deps, _id: String) -> StdResult<MarketResponse> {
        let config = CONFIG.load(deps.storage)?;
        let market_state = MARKET_STATE.load(deps.storage)?;
        let (total_a, total_b) = market_state.total_stakes(&config);
        let (odds_a, odds_b) = market_state.calculate_odds(&config);

        let options_with_odds = vec![
            crate::msg::OptionWithOdds {
                option: config.pairs[0].text.clone(),
                odds: odds_a.to_string(),
                token_denom: config.pairs[0].associated_token_denom.clone(),
                total_staked: total_a.to_string(),
            },
            crate::msg::OptionWithOdds {
                option: config.pairs[1].text.clone(),
                odds: odds_b.to_string(),
                token_denom: config.pairs[1].associated_token_denom.clone(),
                total_staked: total_b.to_string(),
            },
        ];

        Ok(MarketResponse {
            id: config.id,
            options: config.pairs.iter().map(|p| p.text.clone()).collect(),
            status: market_state.status,
            total_value: market_state.total_value,
            num_bettors: market_state.num_bettors,
            token_a: Coin {
                denom: config.pairs[0].associated_token_denom.clone(),
                amount: total_a.to_string(),
            },
            token_b: Coin {
                denom: config.pairs[1].associated_token_denom.clone(),
                amount: total_b.to_string(),
            },
            buy_token: config.buy_token,
            banner_url: config.banner_url,
            description: config.description,
            title: config.title,
            end_time: config.end_time,
            start_time: config.start_time,
            resolution_source: config.resolution_source,
            options_with_odds,
        })
    }
    pub fn query_shares(
        deps: Deps,
        _market_id: String,
        user: Addr,
    ) -> StdResult<AllSharesResponse> {
        let config = CONFIG.load(deps.storage)?;

        // Query shares for both options for this user
        let mut user_shares: Vec<ShareResponse> = Vec::new();

        for option in &config.pairs {
            if let Some(share) = SHARES.may_load(deps.storage, (&user, &option.text))? {
                user_shares.push(ShareResponse {
                    user: user.clone(),
                    option: option.text.clone(),
                    amount: Coin {
                        denom: config.buy_token.clone(),
                        amount: share.amount.to_string(),
                    },
                    has_withdrawn: share.has_withdrawn,
                });
            }
        }

        Ok(AllSharesResponse {
            shares: user_shares,
        })
    }
    pub fn query_market_stats(deps: Deps, _market_id: String) -> StdResult<MarketStatsResponse> {
        let market_state = MARKET_STATE.load(deps.storage)?;
        let config = CONFIG.load(deps.storage)?;

        let (odds_a, odds_b) = market_state.calculate_odds(&config);

        let options_odds = vec![
            crate::msg::OptionOdds {
                option: config.pairs[0].text.clone(),
                odds: odds_a,
            },
            crate::msg::OptionOdds {
                option: config.pairs[1].text.clone(),
                odds: odds_b,
            },
        ];

        Ok(MarketStatsResponse {
            total_value: market_state.total_value,
            num_bettors: market_state.num_bettors,
            options_odds,
            // Keep legacy fields for backward compatibility
            odds_a,
            odds_b,
        })
    }

    pub fn query_user_potential_winnings(
        deps: Deps,
        _market_id: String,
        user: Addr,
    ) -> StdResult<UserPotentialWinningsResponse> {
        let market_state = MARKET_STATE.load(deps.storage)?;
        let config = CONFIG.load(deps.storage)?;

        let (winnings_a, winnings_b) =
            market_state.calculate_potential_winnings(deps.storage, &user, &config)?;

        let options = vec![
            crate::msg::OptionPotentialWinning {
                option: config.pairs[0].text.clone(),
                potential_winnings: winnings_a.clone(),
            },
            crate::msg::OptionPotentialWinning {
                option: config.pairs[1].text.clone(),
                potential_winnings: winnings_b.clone(),
            },
        ];

        Ok(UserPotentialWinningsResponse {
            options,
            // Keep legacy fields for backward compatibility
            potential_win_a: winnings_a,
            potential_win_b: winnings_b,
        })
    }

    pub fn query_user_winnings(
        deps: Deps,
        _market_id: String,
        user: Addr,
    ) -> StdResult<UserWinningsResponse> {
        let market_state = MARKET_STATE.load(deps.storage)?;
        let config = CONFIG.load(deps.storage)?;
        let winnings = market_state.calculate_winnings(deps.storage, &user, &config)?;
        Ok(UserWinningsResponse { winnings })
    }
    pub fn query_balance(
        deps: Deps,
        account: String,
        denom: String,
    ) -> StdResult<QueryBalanceResponse> {
        let request = QueryBalanceRequest { account, denom };
        request.query(&deps.querier)
    }

    pub fn query_tax_rate(deps: Deps, env: Env) -> StdResult<TaxRateResponse> {
        let config = CONFIG.load(deps.storage)?;
        let market_state = MARKET_STATE.load(deps.storage)?;

        let tax_rate = market_state.calculate_time_based_tax(&config, env.block.time);

        Ok(TaxRateResponse { tax_rate })
    }

    pub fn query_simulate_sell(
        deps: Deps,
        env: Env,
        option: String,
        amount: String,
    ) -> StdResult<SimulateSellResponse> {
        let config = CONFIG.load(deps.storage)?;
        let market_state = MARKET_STATE.load(deps.storage)?;

        // Validate option exists
        let _market_option = config
            .pairs
            .iter()
            .find(|p| p.text == option)
            .ok_or_else(|| StdError::generic_err("Invalid option"))?;

        // Parse amount
        let amount_sent = Uint128::from_str(&amount)
            .map_err(|_| StdError::generic_err("Invalid amount format"))?;

        // Calculate tax
        let tax_rate = market_state.calculate_time_based_tax(&config, env.block.time);
        let amount_after_tax =
            market_state.calculate_sell_amount_with_tax(&config, amount_sent, env.block.time);
        let tax_amount = amount_sent - amount_after_tax;

        Ok(SimulateSellResponse {
            amount_sent: amount_sent.to_string(),
            tax_rate,
            tax_amount: tax_amount.to_string(),
            amount_after_tax: amount_after_tax.to_string(),
        })
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let ver = cw2::get_contract_version(deps.storage)?;

    if ver.contract != CONTRACT_NAME {
        return Err(StdError::generic_err("Can only upgrade from same contract type").into());
    }
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
