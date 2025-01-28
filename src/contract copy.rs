#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_json_binary, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Timestamp,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Market, MarketOutcome, Share, State, MARKETS, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:truth-markets-contract-fixed";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
        } => execute::create_market(deps, info, id, options, end_time),
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
    use cosmwasm_std::{Timestamp, Uint128};

    use super::*;

    pub fn create_market(
        deps: DepsMut,
        info: MessageInfo,
        id: String,
        options: Vec<String>,
        end_time: Timestamp,
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

        let market = Market {
            id: id.clone(),
            options,
            shares: vec![],
            resolved: false,
            outcome: MarketOutcome::Unresolved,
            end_time,
            total_value: Coin::new(Uint128::zero(), "earth"),
            num_bettors: 0,
        };

        // Save the market to the MARKETS map
        MARKETS.save(deps.storage, &id, &market)?;

        // Update the state with the new market ID\
        state.admin = info.sender;
        state.market_ids.push(id);
        STATE.save(deps.storage, &state)?;

        Ok(Response::new().add_attribute("action", "create_market"))
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

        // Check if the market has ended
        // if env.block.time >= market.end_time {
        //     return Err(ContractError::Std(StdError::generic_err(
        //         "Market has ended",
        //     )));
        // }

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

        // Add the share to the market
        let share = Share {
            user: info.sender,
            option: option.clone(),
            amount,
        };
        market.shares.push(share);

        // Save the updated market
        MARKETS.save(deps.storage, &market_id, &market)?;

        Ok(Response::new().add_attribute("action", "buy_share"))
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
        let state = STATE.load(deps.storage)?;
        let market = MARKETS.load(deps.storage, &market_id)?;

        if !market.resolved {
            return Err(ContractError::Std(StdError::generic_err(
                "Market is not resolved",
            )));
        }

        let winning_option = match market.outcome {
            MarketOutcome::OptionA => &market.options[0],
            MarketOutcome::OptionB => &market.options[1],
            MarketOutcome::Unresolved => {
                return Err(ContractError::Std(StdError::generic_err(
                    "Market is unresolved",
                )))
            }
        };

        let total_winning_shares: Uint128 = market
            .shares
            .iter()
            .filter(|s| &s.option == winning_option)
            .map(|s| s.amount.amount)
            .sum();

        let total_losing_shares: Uint128 = market
            .shares
            .iter()
            .filter(|s| &s.option != winning_option)
            .map(|s| s.amount.amount)
            .sum();

        let user_shares: Uint128 = market
            .shares
            .iter()
            .filter(|s| s.user == info.sender && &s.option == winning_option)
            .map(|s| s.amount.amount)
            .sum();

        if user_shares.is_zero() {
            return Err(ContractError::Std(StdError::generic_err(
                "No winning shares to withdraw",
            )));
        }

        let user_share_ratio = user_shares / total_winning_shares;
        let user_winnings = user_share_ratio * total_losing_shares;

        // Transfer winnings to the user (pseudo-code, actual implementation depends on your token handling)
        // transfer(&info.sender, Coin { denom: "earth".to_string(), amount: user_winnings })?;

        Ok(Response::new().add_attribute("action", "withdraw"))
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
    }
}
pub mod query {
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
                amount: s.amount.clone(),
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
}

#[cfg(test)]
mod tests {
    use crate::msg::{MarketResponse, UserWinningsResponse};

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, from_json, Addr, Coin, Timestamp, Uint128};

    // Helper function to instantiate the contract
    fn setup_contract(deps: DepsMut) {
        let msg = InstantiateMsg {
            // admin: Addr::unchecked("admin"),
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
        };
        let info = mock_info("admin", &[]);
        let env = mock_env();
        execute(deps, env, info, msg).unwrap();
    }

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("admin", &coins(1000, "earth"));

        let msg = InstantiateMsg {
            // admin: Addr::unchecked("admin"),
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
        };

        instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::CreateMarket {
            id: "market1".to_string(),
            options: vec!["OptionA".to_string(), "OptionB".to_string()],
            end_time: Timestamp::from_seconds(1000),
        };

        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::BuyShare {
            market_id: "market1".to_string(),
            option: "OptionA".to_string(),
            amount: Coin {
                denom: "earth".to_string(),
                amount: Uint128::new(10),
            },
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.attributes, vec![attr("action", "buy_share")]);

        let market = MARKETS.load(&deps.storage, &"market1".to_string()).unwrap();
        assert_eq!(market.shares.len(), 1);
        assert_eq!(market.shares[0].user, Addr::unchecked("user1"));
        assert_eq!(market.shares[0].option, "OptionA".to_string());
        assert_eq!(
            market.shares[0].amount,
            Coin {
                denom: "earth".to_string(),
                amount: Uint128::new(10)
            }
        );
    }

    #[test]
    fn test_resolve() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("admin", &coins(1000, "earth"));

        let msg = InstantiateMsg {
            // admin: Addr::unchecked("admin"),
        };

        instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::CreateMarket {
            id: "market1".to_string(),
            options: vec!["OptionA".to_string(), "OptionB".to_string()],
            end_time: Timestamp::from_seconds(1000),
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
        let info = mock_info("user1", &coins(1000, "earth"));

        let msg = InstantiateMsg {
            // admin: Addr::unchecked("admin"),
        };

        instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::CreateMarket {
            id: "market1".to_string(),
            options: vec!["OptionA".to_string(), "OptionB".to_string()],
            end_time: Timestamp::from_seconds(1000000000),
        };

        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::BuyShare {
            market_id: "market1".to_string(),
            option: "OptionA".to_string(),
            amount: Coin {
                denom: "earth".to_string(),
                amount: Uint128::new(10),
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
        assert_eq!(res.attributes, vec![attr("action", "withdraw")]);
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
            amount: Coin::new(10u128, "earth"),
        };
        let info = mock_info("user1", &coins(10u128, "earth"));
        let env = mock_env();
        execute(deps.as_mut(), env, info, msg).unwrap();

        // User2 buys 2 shares of OptionA
        let msg = ExecuteMsg::BuyShare {
            market_id: "market1".to_string(),
            option: "OptionA".to_string(),
            amount: Coin::new(2u128, "earth"),
        };
        let info = mock_info("user2", &coins(2u128, "earth"));
        let env = mock_env();
        execute(deps.as_mut(), env, info, msg).unwrap();

        // User3 buys 6 shares of OptionB
        let msg = ExecuteMsg::BuyShare {
            market_id: "market1".to_string(),
            option: "OptionB".to_string(),
            amount: Coin::new(6u128, "earth"),
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
        assert_eq!(res.winnings, Coin::new(5u128, "earth")); // (10 / 12) * 6 = 5

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
        assert_eq!(res.winnings, Coin::new(1u128, "earth")); // (2 / 12) * 6 = 1
    }
}
