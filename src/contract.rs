#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Market, MarketOutcome, Share, State, MARKETS, STATE};
use cosmwasm_std::{CosmosMsg, Timestamp, Uint128};

//Coreum related imports
use coreum_wasm_sdk::types::coreum::asset::ft::v1::MsgIssue;
use coreum_wasm_sdk::types::coreum::asset::ft::v1::{
    MsgBurn, MsgMint, QueryTokenRequest, QueryTokenResponse, Token,
};

use coreum_wasm_sdk::types::cosmos::bank::v1beta1::MsgSend;
use coreum_wasm_sdk::types::cosmos::base::v1beta1::Coin;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let state = State {
        admin: info.sender.clone(),
        market_ids: vec![],
    };
    STATE.save(deps.storage, &state)?;
    Ok(Response::new().add_attribute("action", "instantiate"))
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
        } => execute::create_market(
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
        ExecuteMsg::BuyShare {
            market_id,
            option,
            amount,
        } => execute::buy_share(deps, env, info, market_id, option, amount),
        ExecuteMsg::Resolve {
            market_id,
            winning_option,
        } => execute::resolve(deps, env, info, market_id, winning_option),
        ExecuteMsg::Withdraw { market_id } => execute::withdraw(deps, env, info, market_id),
    }
}

pub mod execute {
    use std::str::FromStr;

    use super::*;

    pub fn create_market(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        id: String,
        options: Vec<String>,
        end_time: Timestamp,
        buy_token: String,
        banner_url: String,
        description: String,
        title: String,
        end_time_string: String,
        start_time_string: String,
        resolution_source: String,
    ) -> Result<Response, ContractError> {
        let mut state = STATE.load(deps.storage)?;
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

        let subunit_token_a = format!(
            "truth{}_{}",
            options[0].to_lowercase().replace(" ", "_"),
            id.to_lowercase().replace(" ", "_")
        );

        let symbol_token_a = format!(
            "TM{}{}", // TM prefix for "Truth Markets"
            options[0].replace(" ", ""),
            id.replace(" ", "")
        );

        // Issue two new smart tokens for the market options
        let token_a = MsgIssue {
            issuer: env.contract.address.to_string(),
            symbol: symbol_token_a.clone(),
            subunit: subunit_token_a.clone(),
            precision: 6,
            initial_amount: "0".to_string(),
            description: format!("Token for {} in market {}", options[0], id),
            //Minting & Burning is enabled
            features: vec![0 as i32, 1 as i32],
            burn_rate: "0".to_string(),
            send_commission_rate: "0".to_string(),
            uri: "https://truthmarkets.com".to_string(),
            uri_hash: "".to_string(),
            extension_settings: None,
            dex_settings: None,
        };

        let denom_token_a = format!("{}-{}", subunit_token_a, env.contract.address);

        let subunit_token_b = format!(
            "truth{}_{}",
            options[1].to_lowercase().replace(" ", "_"),
            id.to_lowercase().replace(" ", "_")
        );

        let symbol_token_b = format!(
            "TM{}{}", // TM prefix for "Truth Markets"
            options[1].replace(" ", ""),
            id.replace(" ", "")
        );

        let token_b = MsgIssue {
            issuer: env.contract.address.to_string(),
            symbol: symbol_token_b.clone(),
            subunit: subunit_token_b.clone(),
            precision: 6,
            initial_amount: "0".to_string(),
            description: format!("Token for {} in market {}", options[1], id),
            features: vec![0 as i32, 1 as i32],
            burn_rate: "0".to_string(),
            send_commission_rate: "0".to_string(),
            uri: "https://truthmarkets.com".to_string(),
            uri_hash: "".to_string(),
            extension_settings: None,
            dex_settings: None,
        };

        let denom_token_b = format!("{}-{}", subunit_token_b, env.contract.address);

        let market = Market {
            id: id.clone(),
            options,
            shares: vec![],
            resolved: false,
            outcome: MarketOutcome::Unresolved,
            end_time,
            total_value: Coin {
                denom: buy_token.clone(),
                amount: "0".to_string(),
            },
            num_bettors: 0,
            token_a: denom_token_a,
            token_b: denom_token_b,
            buy_token: buy_token,
            banner_url: banner_url,
            description: description,
            title: title,
            end_time_string: end_time_string,
            start_time_string: start_time_string,
            resolution_source: resolution_source,
        };

        // Save the market to the MARKETS map
        MARKETS.save(deps.storage, &id, &market)?;

        // Update the state with the new market ID\
        state.admin = info.sender;
        state.market_ids.push(id.clone());
        STATE.save(deps.storage, &state)?;

        Ok(Response::new()
            .add_attribute("action", "create_market")
            .add_attribute("market_id", id)
            .add_message(CosmosMsg::Any(token_a.to_any()))
            .add_message(CosmosMsg::Any(token_b.to_any())))
    }

    pub fn buy_share(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        market_id: String,
        option: String,
        amount: Coin,
    ) -> Result<Response, ContractError> {
        let mut market = MARKETS.load(deps.storage, &market_id)?;

        // Ensure the user sent the correct amount of tokens
        let sent_funds = info
            .funds
            .iter()
            .find(|coin| coin.denom == market.buy_token)
            .ok_or_else(|| ContractError::Std(StdError::generic_err("No tokens sent")))?;

        // Check if the amount matches
        if sent_funds.amount.to_string() != amount.amount {
            return Err(ContractError::Std(StdError::generic_err(
                "Incorrect amount of tokens sent",
            )));
        }

        // Check if the market is already resolved
        if market.resolved {
            return Err(ContractError::Std(StdError::generic_err(
                "Market is already resolved",
            )));
        }

        // Check if the option is valid
        if !market.options.contains(&option) {
            return Err(ContractError::Std(StdError::generic_err("Invalid option")));
        }

        // Find existing shares for this user
        let mut user_shares: Vec<_> = market
            .shares
            .iter_mut()
            .filter(|s| s.user == info.sender)
            .collect();

        match user_shares.len() {
            0 => {
                // User has no shares, create a new one
                let share = Share {
                    user: info.sender.clone(),
                    option: option.clone(),
                    token: amount.clone(),
                    has_withdrawn: false,
                };
                market.shares.push(share);
            }
            1 => {
                // User has one share
                let existing_share = &mut user_shares[0];
                if existing_share.option == option {
                    // Same option - update amount
                    existing_share.token.amount = (Uint128::from_str(&existing_share.token.amount)
                        .unwrap()
                        + Uint128::from_str(&amount.amount).unwrap())
                    .to_string();
                } else {
                    // Different option - create second share
                    let share = Share {
                        user: info.sender.clone(),
                        option: option.clone(),
                        token: amount.clone(),
                        has_withdrawn: false,
                    };
                    market.shares.push(share);
                }
            }
            2 => {
                // User already has two shares
                let matching_share = user_shares.iter_mut().find(|s| s.option == option);

                if let Some(share) = matching_share {
                    // Update existing share for the same option
                    share.token.amount = (Uint128::from_str(&share.token.amount).unwrap()
                        + Uint128::from_str(&amount.amount).unwrap())
                    .to_string();
                } else {
                    return Err(ContractError::Std(StdError::generic_err(
                        "User already has two shares with different options",
                    )));
                }
            }
            _ => {
                return Err(ContractError::Std(StdError::generic_err(
                    "Invalid state: User has more than 2 shares",
                )));
            }
        }

        //Check what option the user is buying
        let chosen_option = if option == market.options[0] {
            market.token_a.clone()
        } else {
            market.token_b.clone()
        };

        let mint_msg = MsgMint {
            sender: env.contract.address.to_string(),
            coin: Some(Coin {
                denom: chosen_option.to_string(),
                amount: sent_funds.amount.to_string(),
            }),
            recipient: info.sender.to_string(),
        };

        // Save the updated market
        MARKETS.save(deps.storage, &market_id, &market)?;

        Ok(Response::new()
            .add_attribute("action", "buy_share")
            .add_message(CosmosMsg::Any(mint_msg.to_any())))
    }

    pub fn resolve(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        market_id: String,
        winning_option: String,
    ) -> Result<Response, ContractError> {
        let state = STATE.load(deps.storage)?;

        // Ensure only the admin can resolve the market
        if info.sender != state.admin {
            return Err(ContractError::Std(StdError::generic_err(
                "Unauthorized: Only the admin can resolve markets",
            )));
        }

        let mut market = MARKETS.load(deps.storage, &market_id)?;

        // Check if the market has ended
        if env.block.time < market.end_time {
            return Err(ContractError::Std(StdError::generic_err(
                "Market has not ended yet",
            )));
        }

        // Check if the market is already resolved
        if market.resolved {
            return Err(ContractError::Std(StdError::generic_err(
                "Market is already resolved",
            )));
        }

        // Check if the winning option is valid
        if !market.options.contains(&winning_option) {
            return Err(ContractError::Std(StdError::generic_err(
                "Invalid winning option",
            )));
        }

        // Update the market outcome
        market.resolved = true;
        market.outcome = if winning_option == market.options[0] {
            MarketOutcome::OptionA
        } else {
            MarketOutcome::OptionB
        };

        // Save the updated market
        MARKETS.save(deps.storage, &market_id, &market)?;

        Ok(Response::new().add_attribute("action", "resolve"))
    }

    pub fn withdraw(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        market_id: String,
    ) -> Result<Response, ContractError> {
        let mut market = MARKETS.load(deps.storage, &market_id)?;

        // Check if market is resolved
        if !market.resolved {
            return Err(ContractError::Std(StdError::generic_err(
                "Market is not resolved yet",
            )));
        }

        // Get winning option
        let winning_option = match market.outcome {
            MarketOutcome::OptionA => &market.options[0],
            MarketOutcome::OptionB => &market.options[1],
            MarketOutcome::Unresolved => {
                return Err(ContractError::Std(StdError::generic_err(
                    "Market outcome is not set",
                )))
            }
        };

        // Find user's shares first
        let user_share = market
            .shares
            .iter()
            .find(|s| s.user == info.sender && &s.option == winning_option)
            .ok_or_else(|| StdError::generic_err("No winning shares found for user"))?;

        if user_share.has_withdrawn {
            return Err(ContractError::Std(StdError::generic_err(
                "User has already withdrawn their winnings",
            )));
        }

        // Now calculate totals
        let total_winning_shares: Uint128 = market
            .shares
            .iter()
            .filter(|s| &s.option == winning_option)
            .map(|s| Uint128::from_str(&s.token.amount).unwrap_or_default())
            .sum();

        let total_losing_shares: Uint128 = market
            .shares
            .iter()
            .filter(|s| &s.option != winning_option)
            .map(|s| Uint128::from_str(&s.token.amount).unwrap_or_default())
            .sum();

        // Validate there are shares to distribute
        if total_winning_shares.is_zero() {
            return Err(ContractError::Std(StdError::generic_err(
                "No winning shares in the market",
            )));
        }

        // Calculate user's winnings
        let user_stake = Uint128::from_str(&user_share.token.amount)?;
        let user_share_ratio = user_stake
            .checked_div(total_winning_shares)
            .map_err(|_| ContractError::Std(StdError::generic_err("Division by zero")))?;

        // User gets their original stake back plus their proportion of the losing pool
        let winnings_from_losing_pool = user_share_ratio
            .checked_mul(total_losing_shares)
            .map_err(|_| ContractError::Std(StdError::generic_err("Multiplication overflow")))?;
        let total_winnings = user_stake
            .checked_add(winnings_from_losing_pool)
            .map_err(|_| ContractError::Std(StdError::generic_err("Addition overflow")))?;

        // Mark share as withdrawn
        let share_index = market
            .shares
            .iter()
            .position(|s| s.user == info.sender && &s.option == winning_option)
            .unwrap();
        market.shares[share_index].has_withdrawn = true;

        // Save updated market state
        MARKETS.save(deps.storage, &market_id, &market)?;

        // Create bank transfer message
        let transfer_msg = MsgSend {
            from_address: env.contract.address.to_string(),
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                amount: total_winnings.to_string(),
                denom: market.buy_token.clone(),
            }],
        };

        Ok(Response::new()
            .add_message(CosmosMsg::Any(transfer_msg.to_any()))
            .add_attribute("action", "withdraw")
            .add_attribute("market_id", market_id)
            .add_attribute("user", info.sender)
            .add_attribute("original_stake", user_stake)
            .add_attribute("winnings_from_pool", winnings_from_losing_pool)
            .add_attribute("total_winnings", total_winnings))
    }
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
    }
}
pub mod query {
    use coreum_wasm_sdk::types::coreum::asset::ft::v1::{
        QueryBalanceRequest, QueryBalanceResponse,
    };
    use cosmwasm_std::Addr;

    use crate::msg::{
        MarketResponse, MarketStatsResponse, ShareResponse, UserPotentialWinningsResponse,
        UserWinningsResponse,
    };

    use super::*;

    pub fn query_market(deps: Deps, id: String) -> StdResult<MarketResponse> {
        let market = MARKETS.load(deps.storage, &id)?;
        Ok(MarketResponse {
            id: market.id,
            options: market.options,
            resolved: market.resolved,
            outcome: market.outcome,
            end_time: market.end_time,
            total_value: market.total_value,
            num_bettors: market.num_bettors,
            token_a: market.token_a,
            token_b: market.token_b,
            buy_token: market.buy_token,
            banner_url: market.banner_url,
            description: market.description,
            title: market.title,
            end_time_string: market.end_time_string,
            start_time_string: market.start_time_string,
            resolution_source: market.resolution_source,
        })
    }
    pub fn query_shares(
        deps: Deps,
        market_id: String,
        user: Addr,
    ) -> StdResult<Vec<ShareResponse>> {
        let market = MARKETS.load(deps.storage, &market_id)?;
        let user_shares: Vec<ShareResponse> = market
            .shares
            .iter()
            .filter(|s| s.user == user)
            .map(|s| ShareResponse {
                user: s.user.clone(),
                option: s.option.clone(),
                amount: s.token.clone(),
                has_withdrawn: s.has_withdrawn,
            })
            .collect();
        Ok(user_shares)
    }
    pub fn query_market_stats(deps: Deps, market_id: String) -> StdResult<MarketStatsResponse> {
        let market = MARKETS.load(deps.storage, &market_id)?;
        let (total_a, total_b) = market.total_stakes();

        let odds_a = if total_a.is_zero() {
            0.0
        } else {
            total_b.u128() as f64 / total_a.u128() as f64
        };

        let odds_b = if total_b.is_zero() {
            0.0
        } else {
            total_a.u128() as f64 / total_b.u128() as f64
        };

        Ok(MarketStatsResponse {
            total_value: market.total_value,
            num_bettors: market.num_bettors,
            odds_a,
            odds_b,
        })
    }

    pub fn query_user_potential_winnings(
        deps: Deps,
        market_id: String,
        user: Addr,
    ) -> StdResult<UserPotentialWinningsResponse> {
        let market = MARKETS.load(deps.storage, &market_id)?;
        let (winnings_a, winnings_b) = market.calculate_potential_winnings(&user);
        Ok(UserPotentialWinningsResponse {
            potential_win_a: winnings_a,
            potential_win_b: winnings_b,
        })
    }
    pub fn query_user_winnings(
        deps: Deps,
        market_id: String,
        user: Addr,
    ) -> StdResult<UserWinningsResponse> {
        let market = MARKETS.load(deps.storage, &market_id)?;
        let winnings = market.calculate_winnings(&user);
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

#[cfg(test)]
mod tests {
    use crate::msg::{MarketResponse, UserWinningsResponse};

    use super::*;
    use coreum_wasm_sdk::types::cosmos::base::v1beta1::Coin as SmartToken;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, from_json, Addr, Coin, Timestamp, Uint128};

    // Helper function to instantiate the contract
    fn setup_contract(deps: DepsMut) {
        let msg = InstantiateMsg {
            buy_denom: "earth".to_string(),
        };
        let info = mock_info("admin", &[]);
        let env = mock_env();
        instantiate(deps, env, info, msg).unwrap();
    }

    // Helper function to create a market
    fn create_market(deps: DepsMut, id: &str, end_time: Timestamp) {
        let msg = ExecuteMsg::CreateMarket {
            id: id.to_string(),
            options: vec!["OptionA".to_string(), "OptionB".to_string()],
            end_time,
            buy_token: "usdc".to_string(),
            banner_url: "https://example.com/banner.png".to_string(),
            description: "This is a description".to_string(),
            title: "This is a title".to_string(),
            end_time_string: "2025-01-01".to_string(),
            start_time_string: "2025-01-01".to_string(),
            resolution_source: "https://example.com/resolution.json".to_string(),
        };
        let info = mock_info("admin", &[]);
        let env = mock_env();
        execute(deps, env, info, msg).unwrap();
    }

    #[test]
    fn test_get_market() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        create_market(deps.as_mut(), "market1", Timestamp::from_seconds(1000));
        let market: MarketResponse = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::GetMarket {
                    id: "market1".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(market.id, "market1");
    }

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("admin", &coins(1000, "ucore"));

        let msg = InstantiateMsg {
            buy_denom: "ucore".to_string(),
        };

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let state = STATE.load(&deps.storage).unwrap();
        assert_eq!(state.market_ids.len(), 0);
    }

    #[test]
    fn test_buy_share() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("user1", &coins(1000, "earth"));

        let msg = InstantiateMsg {
            // admin: Addr::unchecked("admin"),
            buy_denom: "earth".to_string(),
        };

        instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::CreateMarket {
            id: "market1".to_string(),
            options: vec!["OptionA".to_string(), "OptionB".to_string()],
            end_time: Timestamp::from_seconds(1000),
            buy_token: "usdc".to_string(),
            banner_url: "None".to_string(),
            description: "None".to_string(),
            title: "None".to_string(),
            end_time_string: "None".to_string(),
            start_time_string: "None".to_string(),
            resolution_source: "None".to_string(),
        };

        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::BuyShare {
            market_id: "market1".to_string(),
            option: "OptionA".to_string(),
            amount: SmartToken {
                denom: "usdc".to_string(),
                amount: "10".to_string(),
            },
        };

        let token = Coin {
            denom: "ucore".to_string(),
            amount: Uint128::from(1000u128),
        };

        let info2 = mock_info("admin", &[token]);

        let res = execute(deps.as_mut(), env.clone(), info2.clone(), msg).unwrap();

        assert_eq!(res.attributes, vec![attr("action", "buy_share")]);

        let market = MARKETS.load(&deps.storage, &"market1".to_string()).unwrap();
        assert_eq!(market.shares.len(), 1);
        assert_eq!(market.shares[0].user, Addr::unchecked("user1"));
        assert_eq!(market.shares[0].option, "OptionA".to_string());
        assert_eq!(
            market.shares[0].token,
            SmartToken {
                denom: "usdc".to_string(),
                amount: "10".to_string()
            }
        );
    }

    #[test]
    fn test_resolve() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("admin", &coins(1000, "earth"));

        let msg = InstantiateMsg {
            buy_denom: "earth".to_string(),
        };

        instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::CreateMarket {
            id: "market1".to_string(),
            options: vec!["OptionA".to_string(), "OptionB".to_string()],
            end_time: Timestamp::from_seconds(1000),
            buy_token: "usdc".to_string(),
            banner_url: "None".to_string(),
            description: "None".to_string(),
            title: "None".to_string(),
            end_time_string: "None".to_string(),
            start_time_string: "None".to_string(),
            resolution_source: "None".to_string(),
        };

        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::Resolve {
            market_id: "market1".to_string(),
            winning_option: "OptionA".to_string(),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.attributes, vec![attr("action", "resolve")]);

        let market = MARKETS.load(&deps.storage, &"market1".to_string()).unwrap();
        assert_eq!(market.resolved, true);
        assert_eq!(market.outcome, MarketOutcome::OptionA);
    }

    #[test]
    fn test_withdraw() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("user1", &coins(1000, "ucore"));

        let msg = InstantiateMsg {
            buy_denom: "earth".to_string(),
        };

        instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::CreateMarket {
            id: "market1".to_string(),
            options: vec!["OptionA".to_string(), "OptionB".to_string()],
            end_time: Timestamp::from_seconds(1000000000),
            buy_token: "usdc".to_string(),
            banner_url: "None".to_string(),
            description: "None".to_string(),
            title: "None".to_string(),
            end_time_string: "None".to_string(),
            start_time_string: "None".to_string(),
            resolution_source: "None".to_string(),
        };

        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::BuyShare {
            market_id: "market1".to_string(),
            option: "OptionA".to_string(),
            amount: SmartToken {
                denom: "usdc".to_string(),
                amount: "10".to_string(),
            },
        };

        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::Resolve {
            market_id: "market1".to_string(),
            winning_option: "OptionA".to_string(),
        };

        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::Withdraw {
            market_id: "market1".to_string(),
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![attr("action", "withdraw"), attr("amount", "0")]
        );
    }

    #[test]
    fn test_only_admin_can_resolve() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Create a market
        let end_time = Timestamp::from_seconds(1000);
        create_market(deps.as_mut(), "market1", end_time);

        // Non-admin tries to resolve the market
        let msg = ExecuteMsg::Resolve {
            market_id: "market1".to_string(),
            winning_option: "OptionA".to_string(),
        };
        let info = mock_info("user1", &[]);
        let env = mock_env();
        let res = execute(deps.as_mut(), env, info, msg);

        // Ensure the non-admin is rejected
        match res {
            Err(ContractError::Std(e)) => assert_eq!(
                e,
                StdError::generic_err("Unauthorized: Only the admin can resolve markets")
            ),
            _ => panic!("Expected unauthorized error"),
        }

        // Admin resolves the market
        let msg = ExecuteMsg::Resolve {
            market_id: "market1".to_string(),
            winning_option: "OptionA".to_string(),
        };
        let info = mock_info("admin", &[]);
        let env = mock_env();
        let res = execute(deps.as_mut(), env, info, msg);

        // Ensure the market is resolved
        assert!(res.is_ok());
        let market: MarketResponse = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::GetMarket {
                    id: "market1".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(market.resolved);
        assert_eq!(market.outcome, MarketOutcome::OptionA);
    }

    #[test]
    fn test_user_winnings() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Create a market
        let end_time = Timestamp::from_seconds(1000);
        create_market(deps.as_mut(), "market1", end_time);

        // User1 buys 10 shares of OptionA
        let msg = ExecuteMsg::BuyShare {
            market_id: "market1".to_string(),
            option: "OptionA".to_string(),
            amount: SmartToken {
                denom: "usdc".to_string(),
                amount: "10".to_string(),
            },
        };
        let info = mock_info("user1", &coins(10u128, "earth"));
        let env = mock_env();
        execute(deps.as_mut(), env, info, msg).unwrap();

        // User2 buys 2 shares of OptionA
        let msg = ExecuteMsg::BuyShare {
            market_id: "market1".to_string(),
            option: "OptionA".to_string(),
            amount: SmartToken {
                denom: "usdc".to_string(),
                amount: "2".to_string(),
            },
        };
        let info = mock_info("user2", &coins(2u128, "earth"));
        let env = mock_env();
        execute(deps.as_mut(), env, info, msg).unwrap();

        // User3 buys 6 shares of OptionB
        let msg = ExecuteMsg::BuyShare {
            market_id: "market1".to_string(),
            option: "OptionB".to_string(),
            amount: SmartToken {
                denom: "usdc".to_string(),
                amount: "6".to_string(),
            },
        };
        let info = mock_info("user3", &coins(6u128, "earth"));
        let env = mock_env();
        execute(deps.as_mut(), env, info, msg).unwrap();

        // Admin resolves the market to OptionA
        let msg = ExecuteMsg::Resolve {
            market_id: "market1".to_string(),
            winning_option: "OptionA".to_string(),
        };
        let info = mock_info("admin", &[]);
        let env = mock_env();
        execute(deps.as_mut(), env, info, msg).unwrap();

        // Query User1's winnings
        let res: UserWinningsResponse = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::GetUserWinnings {
                    market_id: "market1".to_string(),
                    user: Addr::unchecked("user1"),
                },
            )
            .unwrap(),
        )
        .unwrap();

        // Verify User1's winnings
        assert_eq!(
            res.winnings,
            SmartToken {
                denom: "usdc".to_string(),
                amount: "5".to_string(),
            }
        ); // (10 / 12) * 6 = 5

        // Query User2's winnings
        let res: UserWinningsResponse = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::GetUserWinnings {
                    market_id: "market1".to_string(),
                    user: Addr::unchecked("user2"),
                },
            )
            .unwrap(),
        )
        .unwrap();

        // Verify User2's winnings
        assert_eq!(
            res.winnings,
            SmartToken {
                denom: "usdc".to_string(),
                amount: "1".to_string(),
            }
        ); // (2 / 12) * 6 = 1
    }
}
