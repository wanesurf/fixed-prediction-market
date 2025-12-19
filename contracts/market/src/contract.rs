use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw2::set_contract_version;

//TODO: Add query_current_liquidity
//TODO: Add query_odds
//TODO: Add

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

    if msg.options.len() != 2 {
        return Err(ContractError::Std(StdError::generic_err(
            "Markets must have exactly two options",
        )));
    }

    let subunit_token_a = format!(
        "truth{}_{}",
        msg.options[0].to_lowercase().replace(" ", "_"),
        msg.id.to_lowercase().replace(" ", "_")
    );

    let symbol_token_a = format!(
        "TM{}{}", // TM prefix for "Truth Markets"
        msg.options[0].replace(" ", ""),
        msg.id.replace(" ", "")
    );

    // Issue two new smart tokens for the market options
    let issue_token_a = MsgIssue {
        issuer: env.contract.address.to_string(),
        symbol: symbol_token_a.clone(),
        subunit: subunit_token_a.clone(),
        precision: 6,
        initial_amount: "0".to_string(),
        description: format!("Token for {} in market {}", msg.options[0], msg.id),
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
        msg.options[1].to_lowercase().replace(" ", "_"),
        msg.id.to_lowercase().replace(" ", "_")
    );

    let symbol_token_b = format!(
        "TM{}{}", // TM prefix for "Truth Markets"
        msg.options[1].replace(" ", ""),
        msg.id.replace(" ", "")
    );

    let issue_token_b = MsgIssue {
        issuer: env.contract.address.to_string(),
        symbol: symbol_token_b.clone(),
        subunit: subunit_token_b.clone(),
        precision: 6,
        initial_amount: "0".to_string(),
        description: format!("Token for {} in market {}", msg.options[1], msg.id),
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
        text: msg.options[0].clone(),
        associated_token_denom: denom_token_a.clone(),
    };

    let option_b = MarketOption {
        text: msg.options[1].clone(),
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
    };

    MARKET_STATE.save(deps.storage, &market_state)?;
    CONFIG.save(deps.storage, &market_config)?;

    Ok(Response::new()
        .add_attribute("action", "create_market")
        .add_attribute("market_id", msg.id)
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
            winning_option,
        } => resolve(deps, env, info, market_id, winning_option),
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

    // Update aggregate totals
    if market_option.text == config.pairs[0].text {
        market_state.total_stake_option_a -= amount_sent;
    } else {
        market_state.total_stake_option_b -= amount_sent;
    }

    // Update total value
    let new_total_value =
        Uint128::from_str(&market_state.total_value.amount).unwrap() - amount_sent;
    market_state.total_value.amount = new_total_value.to_string();

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

    Ok(Response::new()
        .add_attribute("action", "sell_share")
        .add_attribute("option", market_option.text)
        .add_attribute("amount", amount_sent.to_string())
        .add_message(CosmosMsg::Any(burn_msg.to_any())))
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
                    share.amount += payment;
                    Ok(share)
                }
                None => Ok(Share {
                    amount: payment,
                    has_withdrawn: false,
                }),
            }
        },
    )?;

    // Update aggregate totals
    if market_option.text == config.pairs[0].text {
        market_state.total_stake_option_a += payment;
    } else {
        market_state.total_stake_option_b += payment;
    }

    // Update total value
    let new_total_value = Uint128::from_str(&market_state.total_value.amount).unwrap() + payment;
    market_state.total_value.amount = new_total_value.to_string();

    // Save the updated market state
    MARKET_STATE.save(deps.storage, &market_state)?;

    // Mint tokens to user
    let mint_msg = MsgMint {
        sender: env.contract.address.to_string(),
        coin: Some(Coin {
            denom: market_option.associated_token_denom.clone(),
            amount: payment.to_string(),
        }),
        recipient: info.sender.to_string(),
    };

    Ok(Response::new()
        .add_attribute("action", "buy_share")
        .add_attribute("option", market_option.text)
        .add_attribute("amount", payment.to_string())
        .add_message(CosmosMsg::Any(mint_msg.to_any())))
}

pub fn resolve(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _market_id: String,
    winning_option: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut market_state = MARKET_STATE.load(deps.storage)?;

    // Ensure only the admin can resolve the market --> The relayer
    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only the admin can resolve markets",
        )));
    }

    // Check if the market has ended
    // if env.block.time < market.end_time {
    //     return Err(ContractError::Std(StdError::generic_err(
    //         "Market has not ended yet",
    //     )));
    // }

    // Check if the market is already resolved
    if matches!(market_state.status, MarketStatus::Resolved(_)) {
        return Err(ContractError::Std(StdError::generic_err(
            "Market is already resolved",
        )));
    }

    let winning_option_obj = config
        .pairs
        .iter()
        .find(|p| p.text == winning_option)
        .cloned()
        .ok_or_else(|| StdError::generic_err("Invalid winning option"))?;

    // Update the market status with the winning option
    market_state.status = MarketStatus::Resolved(winning_option_obj);

    // Save the updated market state
    MARKET_STATE.save(deps.storage, &market_state)?;

    Ok(Response::new().add_attribute("action", "resolve"))
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

    Ok(Response::new()
        .add_message(CosmosMsg::Any(transfer_msg.to_any()))
        .add_attribute("action", "withdraw")
        .add_attribute("market_id", market_id)
        .add_attribute("user", info.sender)
        .add_attribute("total_winnings", total_winnings.amount))
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
    }
}
pub mod query {
    use coreum_wasm_sdk::types::coreum::asset::ft::v1::{
        QueryBalanceRequest, QueryBalanceResponse,
    };
    use cosmwasm_std::Addr;

    use crate::msg::{
        AllSharesResponse, MarketResponse, MarketStatsResponse, OddsResponse, ShareResponse,
        TotalSharesPerOptionResponse, TotalValueResponse, UserPotentialWinningsResponse,
        UserWinningsResponse,
    };

    use super::*;

    pub fn query_odds(deps: Deps) -> StdResult<OddsResponse> {
        let market_state = MARKET_STATE.load(deps.storage)?;
        let config = CONFIG.load(deps.storage)?;
        let (odds_a, odds_b) = market_state.calculate_odds(&config);

        use crate::msg::OptionOdds;
        Ok(OddsResponse {
            odds: vec![
                OptionOdds {
                    option: config.pairs[0].text.clone(),
                    odds: odds_a,
                },
                OptionOdds {
                    option: config.pairs[1].text.clone(),
                    odds: odds_b,
                },
            ],
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

        Ok(TotalSharesPerOptionResponse {
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

        Ok(MarketStatsResponse {
            total_value: market_state.total_value,
            num_bettors: market_state.num_bettors,
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

        Ok(UserPotentialWinningsResponse {
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
