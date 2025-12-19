#[cfg(test)]
mod tests {
    use std::ops::Mul;
    use std::str::FromStr;

    use chrono::Utc;
    use coreum_test_tube::{Account, Bank, CoreumTestApp, Module, Runner, SigningAccount, Wasm};
    use coreum_wasm_sdk::types::cosmos::bank::v1beta1::QueryBalanceRequest;
    use coreum_wasm_sdk::types::cosmwasm::wasm::v1::{
        QueryContractInfoRequest, QueryContractInfoResponse,
    };

    use cosmwasm_std::{coin, Addr, Decimal, Timestamp, Uint128};
    use market::msg::{
        AllSharesResponse, ExecuteMsg, MarketResponse, MarketStatsResponse, OddsResponse, QueryMsg,
        TotalSharesPerOptionResponse, TotalValueResponse, UserPotentialWinningsResponse,
        UserWinningsResponse,
    };
    use market::state::{MarketOption, MarketStatus};
    use registry::msg::{
        ExecuteMsg as RegistryExecuteMsg, InstantiateMsg as RegistryInstantiateMsg,
        QueryMsg as RegistryQueryMsg,
    };
    use registry::state::MarketInfo;

    const FEE_DENOM: &str = "ucore";
    const BUY_TOKEN: &str = "uusdc";
    const COMMISSION_RATE: f64 = 0.05;

    fn setup_registry_and_market(
        wasm: &Wasm<'_, CoreumTestApp>,
        admin: &SigningAccount,
        oracle: &Addr,
    ) -> (String, String) {
        let market_wasm_byte_code = std::fs::read("../../artifacts/market.wasm").unwrap();
        let registry_wasm_byte_code = std::fs::read("../../artifacts/registry.wasm").unwrap();

        let market_code_id = wasm
            .store_code(&market_wasm_byte_code, None, admin)
            .unwrap()
            .data
            .code_id;

        let registry_code_id = wasm
            .store_code(&registry_wasm_byte_code, None, admin)
            .unwrap()
            .data
            .code_id;

        // Instantiate registry
        let registry_address = wasm
            .instantiate(
                registry_code_id,
                &RegistryInstantiateMsg {
                    oracle: oracle.clone(),
                    commission_rate: Decimal::from_str(&COMMISSION_RATE.to_string()).unwrap(), // 5%
                    market_code_id,
                },
                Some(&admin.address()),
                Some("test_registry"),
                &[],
                admin,
            )
            .unwrap()
            .data
            .address;

        let now = Timestamp::from_seconds(Utc::now().timestamp() as u64);
        let end_time = now.plus_seconds(3600 * 24 * 1); // 1 days from now
                                                        // let start_time = now.minus_seconds(3600 * 24 * 30); // 30 days ago

        // Create market through registry
        let create_market_res = wasm
            .execute(
                &registry_address,
                &RegistryExecuteMsg::CreateMarket {
                    id: "test_market_1".to_string(),
                    options: vec!["Yes".to_string(), "No".to_string()],
                    start_time: now,
                    end_time: end_time,
                    buy_token: BUY_TOKEN.to_string(),
                    banner_url: "https://example.com/banner.png".to_string(),
                    description: "Test prediction market for integration testing".to_string(),
                    title: "Test Market".to_string(),
                    resolution_source: "https://example.com/resolution".to_string(),
                    oracle: oracle.clone(),
                },
                &[coin(20_000_000, FEE_DENOM)], // Required payment for market creation
                admin,
            )
            .unwrap();

        // Extract market address from events
        let market_address = create_market_res
            .events
            .iter()
            .find_map(|e| {
                if e.ty == "instantiate" {
                    e.attributes.iter().find_map(|attr| {
                        if attr.key == "_contract_address" {
                            Some(attr.value.clone())
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            })
            .unwrap();

        let market_info: MarketInfo = wasm
            .query(
                &registry_address,
                &RegistryQueryMsg::Market {
                    market_id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        let market_address_from_registry = market_info.contract_address;

        println!(
            "Market address from registry: {}",
            market_address_from_registry
        );
        println!("Market address from events: {}", market_address);

        (registry_address, market_address)
    }

    #[test]
    fn test_market_creation_through_registry() {
        let app = CoreumTestApp::new();
        let admin = app
            .init_account(&[coin(100_000_000_000_000_000_000u128, FEE_DENOM)])
            .unwrap();
        let oracle = app
            .init_account(&[coin(100_000_000_000_000_000_000u128, FEE_DENOM)])
            .unwrap();
        let wasm: Wasm<'_, CoreumTestApp> = Wasm::new(&app);

        let (registry_address, market_address) =
            setup_registry_and_market(&wasm, &admin, &Addr::unchecked(oracle.address()));

        println!("Registry address: {}", registry_address);
        println!("Market address: {}", market_address);

        // Verify the market was created correctly through registry
        let market_info: MarketInfo = wasm
            .query(
                &registry_address,
                &RegistryQueryMsg::Market {
                    market_id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        assert_eq!(market_info.id, "test_market_1");
        //verify we have the right market contract address in the registry
        assert_eq!(market_info.contract_address.to_string(), market_address);
        assert_eq!(market_info.pairs.len(), 2);
        assert_eq!(market_info.pairs[0].text, "Yes");
        assert_eq!(market_info.pairs[1].text, "No");

        let market: MarketResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetMarket {
                    id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        assert_eq!(market.id, "test_market_1");
        assert_eq!(market.options, vec!["Yes".to_string(), "No".to_string()]);
        assert_eq!(market.status, MarketStatus::Pending);
        assert_eq!(market.buy_token, BUY_TOKEN);
        assert_eq!(market.title, "Test Market");
    }

    #[test]
    fn test_buy_shares_through_registry_created_market() {
        let app = CoreumTestApp::new();
        let admin = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let oracle = app
            .init_account(&[coin(100_000_000_000_000_000_000u128, FEE_DENOM)])
            .unwrap();
        let user1 = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let wasm: Wasm<'_, CoreumTestApp> = Wasm::new(&app);

        let (_registry_address, market_address) =
            setup_registry_and_market(&wasm, &admin, &Addr::unchecked(oracle.address()));

        // User1 buys shares for "Yes"
        let buy_res = wasm
            .execute(
                &market_address,
                &ExecuteMsg::BuyShare {
                    market_id: "test_market_1".to_string(),
                    option: "Yes".to_string(),
                },
                &[coin(1000, BUY_TOKEN)],
                &user1,
            )
            .unwrap();

        // Verify the transaction was successful
        assert!(buy_res.events.iter().any(|e| e.ty == "wasm"));

        // Query user's shares
        let shares: AllSharesResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetShares {
                    market_id: "test_market_1".to_string(),
                    user: Addr::unchecked(user1.address()),
                },
            )
            .unwrap();

        assert_eq!(shares.shares.len(), 1);
        assert_eq!(shares.shares[0].user, Addr::unchecked(user1.address()));
        assert_eq!(shares.shares[0].option, "Yes");
        assert_eq!(shares.shares[0].amount.amount, "1000");
        assert_eq!(shares.shares[0].has_withdrawn, false);
    }

    #[test]
    fn test_multiple_users_buying_shares() {
        let app = CoreumTestApp::new();
        let admin = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let oracle = app
            .init_account(&[coin(100_000_000_000_000_000_000u128, FEE_DENOM)])
            .unwrap();
        let user1 = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let user2 = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let wasm: Wasm<'_, CoreumTestApp> = Wasm::new(&app);

        let (_registry_address, market_address) =
            setup_registry_and_market(&wasm, &admin, &Addr::unchecked(oracle.address()));

        // User1 buys "Yes" shares
        wasm.execute(
            &market_address,
            &ExecuteMsg::BuyShare {
                market_id: "test_market_1".to_string(),
                option: "Yes".to_string(),
            },
            &[coin(1000, BUY_TOKEN)],
            &user1,
        )
        .unwrap();

        // User2 buys "No" shares
        wasm.execute(
            &market_address,
            &ExecuteMsg::BuyShare {
                market_id: "test_market_1".to_string(),
                option: "No".to_string(),
            },
            &[coin(2000, BUY_TOKEN)],
            &user2,
        )
        .unwrap();

        // Query market stats
        let stats: MarketStatsResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetMarketStats {
                    market_id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        assert_eq!(stats.total_value.amount, "3000");
        assert_eq!(stats.num_bettors, 2);

        // Query total shares per option
        let total_shares: TotalSharesPerOptionResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetTotalSharesPerOption {
                    market_id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        assert_eq!(total_shares.amount_a.amount, "1000"); // Yes option
        assert_eq!(total_shares.amount_b.amount, "2000"); // No option

        // Query the odds
        let odds: OddsResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetOdds {
                    market_id: "test_market_1".to_string(),
                },
            )
            .unwrap();
        println!(
            "{} odds: {}, {} odds: {}",
            odds.odds[0].option, odds.odds[0].odds, odds.odds[1].option, odds.odds[1].odds
        );
        assert!(odds.odds[0].odds > Decimal::zero());
        assert!(odds.odds[1].odds > Decimal::zero());
    }

    #[test]
    fn test_market_resolution_and_withdrawal() {
        let app = CoreumTestApp::new();
        let admin = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let oracle = app
            .init_account(&[coin(100_000_000_000_000_000_000u128, FEE_DENOM)])
            .unwrap();
        let user1 = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(1000u128, BUY_TOKEN),
            ])
            .unwrap();
        let user2 = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(2000u128, BUY_TOKEN),
            ])
            .unwrap();
        let wasm: Wasm<'_, CoreumTestApp> = Wasm::new(&app);
        let bank = Bank::new(&app);

        let (registry_address, market_address) =
            setup_registry_and_market(&wasm, &admin, &Addr::unchecked(oracle.address()));

        // Users buy shares
        let user1_betting_amount = 1000;
        let user2_betting_amount = 2000;
        wasm.execute(
            &market_address,
            &ExecuteMsg::BuyShare {
                market_id: "test_market_1".to_string(),
                option: "Yes".to_string(),
            },
            &[coin(user1_betting_amount, BUY_TOKEN)],
            &user1,
        )
        .unwrap();

        wasm.execute(
            &market_address,
            &ExecuteMsg::BuyShare {
                market_id: "test_market_1".to_string(),
                option: "No".to_string(),
            },
            &[coin(user2_betting_amount, BUY_TOKEN)],
            &user2,
        )
        .unwrap();

        // Get market info to find the token denoms
        let market: MarketResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetMarket {
                    id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        // Registry (admin) resolves market to "Yes"
        wasm.execute(
            &market_address,
            &ExecuteMsg::Resolve {
                market_id: "test_market_1".to_string(),
                winning_option: "Yes".to_string(),
            },
            &[],
            &admin, // Registry is the admin that can resolve markets
        )
        .unwrap();

        // (potential winnings because we are not considering the commission)
        let potential_winnings: UserPotentialWinningsResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetUserPotentialWinnings {
                    market_id: "test_market_1".to_string(),
                    user: Addr::unchecked(user1.address()),
                },
            )
            .unwrap();

        println!(
            "User1 potential winnings: {}",
            potential_winnings.potential_win_a.amount
        );
        // we take the fees on the total whitdraw! here: (1000+2000) * (1-COMMISSION_RATE) = 2850 uusdc
        assert_eq!(
            Uint128::from_str(&potential_winnings.potential_win_a.amount).unwrap(),
            Uint128::from_str(&"2850".to_string()).unwrap()
        );

        // Check market status after resolution
        let market_after: MarketResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetMarket {
                    id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        if let MarketStatus::Resolved(winning_option) = market_after.status {
            assert_eq!(winning_option.text, "Yes");
        } else {
            panic!("Market should be resolved");
        }

        // Query user1's winnings (they bet on the winning option)
        let winnings: UserWinningsResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetUserWinnings {
                    market_id: "test_market_1".to_string(),
                    user: Addr::unchecked(user1.address()),
                },
            )
            .unwrap();

        // User1 should have winnings since they bet on "Yes" which won
        println!("User1 winnings: {}", winnings.winnings.amount);
        assert!(Uint128::from_str(&winnings.winnings.amount).unwrap() > Uint128::zero());

        // Get user1's balance of the winning token before withdrawal
        let user1_balance_before = bank
            .query_balance(&QueryBalanceRequest {
                address: user1.address().to_string(),
                denom: market.token_a.denom.clone(), // "Yes" token
            })
            .unwrap()
            .balance
            .unwrap()
            .amount;

        //User 2 try to withdraw their winnings (even though he lost)
        // let result = wasm.execute(
        //     &market_address,
        //     &ExecuteMsg::Withdraw {
        //         market_id: "test_market_1".to_string(),
        //     },
        //     &[coin(
        //         Uint128::from_str(&user2_balance_before).unwrap().u128(),
        //         &market.token_b.denom,
        //     )],
        //     &user2,
        // );

        // User1 withdraws their winnings by sending the winning token
        wasm.execute(
            &market_address,
            &ExecuteMsg::Withdraw {
                market_id: "test_market_1".to_string(),
            },
            &[coin(user1_betting_amount, &market.token_a.denom)],
            &user1,
        )
        .unwrap();

        // Check that user1's balance of buy_token increased
        let user1_balance_after = bank
            .query_balance(&QueryBalanceRequest {
                address: user1.address().to_string(),
                denom: BUY_TOKEN.to_string(),
            })
            .unwrap()
            .balance
            .unwrap()
            .amount;

        //should be, he's bet+the losing side - commission
        println!("User1 balance after withdrawal: {}", user1_balance_after);
        let expected_balance = ((1000.0 + 2000.0) * (1.0 - COMMISSION_RATE)) as u128;
        assert_eq!(
            Uint128::from_str(&user1_balance_after).unwrap(),
            Uint128::from_str(&expected_balance.to_string()).unwrap()
        );
    }

    #[test]
    fn test_odds_calculation() {
        let app = CoreumTestApp::new();
        let admin = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let oracle = app
            .init_account(&[coin(100_000_000_000_000_000_000u128, FEE_DENOM)])
            .unwrap();
        let user1 = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let user2 = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let wasm: Wasm<'_, CoreumTestApp> = Wasm::new(&app);

        let (_registry_address, market_address) =
            setup_registry_and_market(&wasm, &admin, &Addr::unchecked(oracle.address()));

        // User1 buys "Yes" shares
        wasm.execute(
            &market_address,
            &ExecuteMsg::BuyShare {
                market_id: "test_market_1".to_string(),
                option: "Yes".to_string(),
            },
            &[coin(3000, BUY_TOKEN)],
            &user1,
        )
        .unwrap();

        // User2 buys "No" shares
        wasm.execute(
            &market_address,
            &ExecuteMsg::BuyShare {
                market_id: "test_market_1".to_string(),
                option: "No".to_string(),
            },
            &[coin(1000, BUY_TOKEN)],
            &user2,
        )
        .unwrap();

        // Query odds
        let odds: OddsResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetOdds {
                    market_id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        println!(
            "{} odds: {}, {} odds: {}",
            odds.odds[0].option, odds.odds[0].odds, odds.odds[1].option, odds.odds[1].odds
        );

        // With 3000 on "Yes" and 1000 on "No":
        // odds_a (Yes) = 1000/3000 = 0.333...
        // odds_b (No) = 3000/1000 = 3.0
        assert_eq!(
            odds.odds[0].odds,
            Decimal::from_str("0.333333333333333333").unwrap()
        );
        assert_eq!(odds.odds[1].odds, Decimal::from_str("3").unwrap());
    }

    #[test]
    fn test_unauthorized_resolution() {
        let app = CoreumTestApp::new();
        let admin = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let oracle = app
            .init_account(&[coin(100_000_000_000_000_000_000u128, FEE_DENOM)])
            .unwrap();
        let user1 = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let wasm: Wasm<'_, CoreumTestApp> = Wasm::new(&app);

        let (_registry_address, market_address) =
            setup_registry_and_market(&wasm, &admin, &Addr::unchecked(oracle.address()));

        // Try to resolve market as non-admin user (should fail)
        let result = wasm.execute(
            &market_address,
            &ExecuteMsg::Resolve {
                market_id: "test_market_1".to_string(),
                winning_option: "Yes".to_string(),
            },
            &[],
            &user1, // Non-admin user
        );

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Unauthorized"));
    }

    #[test]
    fn test_potential_winnings_calculation() {
        let app = CoreumTestApp::new();
        let admin = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let oracle = app
            .init_account(&[coin(100_000_000_000_000_000_000u128, FEE_DENOM)])
            .unwrap();
        let user1 = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let user2 = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let wasm: Wasm<'_, CoreumTestApp> = Wasm::new(&app);

        let (_registry_address, market_address) =
            setup_registry_and_market(&wasm, &admin, &Addr::unchecked(oracle.address()));

        // User1 buys "Yes" shares
        wasm.execute(
            &market_address,
            &ExecuteMsg::BuyShare {
                market_id: "test_market_1".to_string(),
                option: "Yes".to_string(),
            },
            &[coin(2000, BUY_TOKEN)],
            &user1,
        )
        .unwrap();

        // User2 buys "No" shares
        wasm.execute(
            &market_address,
            &ExecuteMsg::BuyShare {
                market_id: "test_market_1".to_string(),
                option: "No".to_string(),
            },
            &[coin(1000, BUY_TOKEN)],
            &user2,
        )
        .unwrap();

        // Query user1's potential winnings
        let potential_winnings: UserPotentialWinningsResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetUserPotentialWinnings {
                    market_id: "test_market_1".to_string(),
                    user: Addr::unchecked(user1.address()),
                },
            )
            .unwrap();

        println!(
            "User1 potential winnings A: {}",
            potential_winnings.potential_win_a.amount
        );
        println!(
            "User1 potential winnings B: {}",
            potential_winnings.potential_win_b.amount
        );

        // User1 has 2000 on "Yes", and there's 1000 on "No"
        // After commission (5%), user stake becomes 2000 * 0.95 = 1900
        // Odds for "Yes" = 1000/2000 = 0.5
        // Potential winnings = (1900 * 0.5) + 1900 = 950 + 1900 = 2850
        let expected_winnings_a = Uint128::from(2850u128);
        assert_eq!(
            Uint128::from_str(&potential_winnings.potential_win_a.amount).unwrap(),
            expected_winnings_a
        );
    }

    #[test]
    fn test_buying_shares_after_resolved_fails() {
        let app = CoreumTestApp::new();
        let admin = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let oracle = app
            .init_account(&[coin(100_000_000_000_000_000_000u128, FEE_DENOM)])
            .unwrap();
        let user1 = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let wasm: Wasm<'_, CoreumTestApp> = Wasm::new(&app);

        let (_registry_address, market_address) =
            setup_registry_and_market(&wasm, &admin, &Addr::unchecked(oracle.address()));

        // Resolve the market first
        wasm.execute(
            &market_address,
            &ExecuteMsg::Resolve {
                market_id: "test_market_1".to_string(),
                winning_option: "Yes".to_string(),
            },
            &[],
            &admin,
        )
        .unwrap();

        // Try to buy shares after resolution (should fail)
        let result = wasm.execute(
            &market_address,
            &ExecuteMsg::BuyShare {
                market_id: "test_market_1".to_string(),
                option: "Yes".to_string(),
            },
            &[coin(1000, BUY_TOKEN)],
            &user1,
        );

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Market is already resolved"));
    }
}
