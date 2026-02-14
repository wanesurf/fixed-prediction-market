mod helpers {
    pub mod keys {
        use clp_feed_interface::msg::PriceSubmission;
        use ed25519_dalek::{Signature, SigningKey};
        use signature::SignerMut;

        #[derive(Clone)]
        pub struct ValidatorKey {
            pub signing_key: SigningKey,
            pub public_key: String,
            pub id: String,
        }

        impl ValidatorKey {
            pub fn sign_price_submission(&mut self, validator_id: &str, asset: &str, price: &str, timestamp: u64, sources: &[String]) -> Result<String, signature::Error> {
                let tx = format!(
                    "{}:{}:{}:{}:{}",
                    validator_id, asset, price, timestamp, sources.join(",")
                );
                let signature = self.signing_key.sign(tx.as_bytes());
                Ok(hex::encode(signature.to_bytes()))
            }
        }

        pub fn signing_key_from_seed(seed: &str, id: &str) -> ValidatorKey {
            let key_bytes = hex::decode(seed)
                .expect("Invalid private key: must be a valid hex string");
                
            let signing_key = SigningKey::from_bytes(
                &key_bytes.as_slice().try_into()
                    .expect("Invalid private key: failed to convert to signing key")
            );

            let public_key = hex::encode(signing_key.verifying_key().to_bytes());

            ValidatorKey {
                signing_key,
                public_key,
                id: id.to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::Utc;
    use clp_feed_interface::msg::{AggregatedPrice, AssetInfo, PriceSubmission};
    use coreum_test_tube::{Account, Bank, CoreumTestApp, Module, SigningAccount, Wasm};
    use coreum_wasm_sdk::types::cosmos::bank::v1beta1::{MsgSend, QueryBalanceRequest};

    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{coin, Addr, Decimal, Timestamp, Uint128};
    use market::msg::{
        AllSharesResponse, ExecuteMsg, MarketResponse, MarketStatsResponse, MarketType, OddsResponse, QueryMsg, SimulateSellResponse, TaxRateResponse, TotalSharesPerOptionResponse, TotalValueResponse, UserPotentialWinningsResponse, UserWinningsResponse
    };
    use market::state::MarketStatus;
    use registry::msg::{
        ExecuteMsg as RegistryExecuteMsg, InstantiateMsg as RegistryInstantiateMsg,
        QueryMsg as RegistryQueryMsg,
    };
    use registry::state::MarketInfo;
    
    use super::helpers::keys::signing_key_from_seed;

    const FEE_DENOM: &str = "ucore";
    const BUY_TOKEN: &str = "uusdc";
    const COMMISSION_RATE_BPS: u128 = 500; // 5% in basis points

    // Helper function to calculate net amount after commission
    fn calculate_net_amount(amount: u128) -> u128 {
        amount - (amount * COMMISSION_RATE_BPS / 10000)
    }

    // Helper function to get current timestamp
    fn get_start_time() -> Timestamp {
        Timestamp::from_seconds(Utc::now().timestamp() as u64)
    }

    // Helper function to get end time (1 day from now)
    fn get_end_time() -> Timestamp {
        let now = Timestamp::from_seconds(Utc::now().timestamp() as u64);
        now.plus_seconds(3600 * 24 * 1)
    }

    fn setup_registry_and_market(
        wasm: &Wasm<'_, CoreumTestApp>,
        admin: &SigningAccount,
    ) -> (String, String) {
        let market_wasm_byte_code = std::fs::read("../../artifacts/market.wasm").unwrap();
        let registry_wasm_byte_code = std::fs::read("../../artifacts/registry.wasm").unwrap();
        let clp_feed_wasm_byte_code = std::fs::read("../../artifacts/clp_feed.wasm").unwrap();

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

        let clp_feed_code_id = wasm
            .store_code(&clp_feed_wasm_byte_code, None, admin)
            .unwrap()
            .data
            .code_id;

        #[cw_serde]

        pub struct ClpFeedInstantiateMsg {
            pub admin: String,
            pub validators: Vec<(String, String)>, // (validator_id, public_key_hex)
            pub operators: Vec<String>,
            pub min_signatures: Option<u32>,
            pub max_price_age: Option<u64>,
            pub assets: Vec<AssetInfo>,
        }


    let validator1_key = signing_key_from_seed("0000000000000000000000000000000000000000000000000000000000000001", "validator_1");
    let validator2_key = signing_key_from_seed("0000000000000000000000000000000000000000000000000000000000000002", "validator_2");


    // Instantiate feed
    let feed_addr = wasm
        .instantiate(
            clp_feed_code_id,
            &ClpFeedInstantiateMsg {
                admin: admin.address().to_string(),
                validators: vec![
                    ("validator_1".to_string(), validator1_key.clone().public_key),
                    ("validator_2".to_string(), validator2_key.clone().public_key),
                ],
                operators: vec![admin.address().to_string()],
                min_signatures: Some(1),
                max_price_age: Some(300),
                assets: vec![
                    AssetInfo {
                        name: "CORE".to_string(),
                        denom: "ucore".to_string(),
                        decimals: 6,
                    },
                ],
            },
            Some(&admin.address()),
            Some("clp-feed"),
            &[],
            admin,
        )
        .unwrap()
        .data
        .address;

          //set price in clp_feed contract 

    wasm.execute(
        &feed_addr,
        &clp_feed_interface::msg::ExecuteMsg::SubmitPrice {
            aggregated_price: AggregatedPrice {
                asset: "CORE".to_string(),
                price: "1.0".to_string(),
                timestamp: get_start_time(),
                deviation: None,
                submissions: vec![],
                feed_block_height: 0, //TODO: get from block height
                chain_block_height: None, //TODO: get from block height
            },
            validator_submissions: vec![validator1_key.clone(), validator2_key.clone()].iter_mut().map(|validator_key| {
                let asset = "CORE";
                let price = "1.0";
                let timestamp = get_start_time().seconds();
                let sources = vec![];

                let signature = validator_key.sign_price_submission(
                    &validator_key.id.clone(),
                    asset,
                    price,
                    timestamp,
                    &sources
                ).unwrap();

                PriceSubmission {
                    validator_id: validator_key.id.clone(),
                    asset: asset.to_string(),
                    price: price.to_string(),
                    timestamp,
                    sources,
                    signature,
                }
            }).collect(),
        },
        &[],
        admin,
    ).unwrap();


        // Instantiate registry
        let registry_address = wasm
            .instantiate(
                registry_code_id,
                &RegistryInstantiateMsg {
                    oracle: Addr::unchecked(feed_addr.clone()),
                    commission_rate: Uint128::from(COMMISSION_RATE_BPS), // 5% in BPS
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

        //TODO instantiate clp_feed contract

        // Create market through registry
        let create_market_res = wasm
            .execute(
                &registry_address,
                &RegistryExecuteMsg::CreateMarket {
                    id: "test_market_1".to_string(),
                    options: vec!["Yes".to_string(), "No".to_string()],
                    start_time: get_start_time(),
                    end_time: get_end_time(),
                    buy_token: BUY_TOKEN.to_string(),
                    banner_url: "https://example.com/banner.png".to_string(),
                    description: "Test prediction market for integration testing".to_string(),
                    title: "Test Market".to_string(),
                    resolution_source: "https://example.com/resolution".to_string(),
                    oracle: Addr::unchecked(feed_addr.clone()),
                    asset_to_track: "CORE".to_string(),
                    market_type: MarketType::UpDown,
                    target_price: Decimal::from_str("1.5").unwrap(), // Target price higher than initial price
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
            setup_registry_and_market(&wasm, &admin);

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
            setup_registry_and_market(&wasm, &admin);

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

        // Verify the transaction was successful - check for any event indicating success
        assert!(buy_res.events.len() > 0, "No events found in buy response");

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
        assert_eq!(shares.shares[0].amount.amount, calculate_net_amount(1000).to_string());
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
            setup_registry_and_market(&wasm, &admin);

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

        // Total value = net amount after commission for both users
        let expected_total = calculate_net_amount(1000) + calculate_net_amount(2000);
        assert_eq!(stats.total_value.amount, expected_total.to_string());
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

        assert_eq!(total_shares.amount_a.amount, calculate_net_amount(1000).to_string());
        assert_eq!(total_shares.amount_b.amount, calculate_net_amount(2000).to_string());

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
            setup_registry_and_market(&wasm, &admin);

        // Users buy shares
        let user1_betting_amount = 1000;
        let user1_net_tokens = calculate_net_amount(user1_betting_amount); // Tokens user actually received
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
        // Commission is now taken during buy/sell operations, not on withdrawal
        // So full amount should be available for withdrawal
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
            &[coin(user1_net_tokens, &market.token_a.denom)],
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
        // Commission is now taken during buy/sell, so the net amounts were already reduced
        let expected_balance = calculate_net_amount(user1_betting_amount) + calculate_net_amount(user2_betting_amount);
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
            setup_registry_and_market(&wasm, &admin);

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
            setup_registry_and_market(&wasm, &admin);

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
            setup_registry_and_market(&wasm, &admin);

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
            setup_registry_and_market(&wasm, &admin);

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

    #[test]
    fn test_sell_shares_functionality() {
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
        let bank = Bank::new(&app);

        let (_registry_address, market_address) =
            setup_registry_and_market(&wasm, &admin);

        // User1 buys shares for "Yes"
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

        // Get market info to find token denom
        let market: MarketResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetMarket {
                    id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        // Check user's shares before selling
        let shares_before: AllSharesResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetShares {
                    market_id: "test_market_1".to_string(),
                    user: Addr::unchecked(user1.address()),
                },
            )
            .unwrap();

        assert_eq!(shares_before.shares.len(), 1);
        assert_eq!(shares_before.shares[0].option, "Yes");
        assert_eq!(shares_before.shares[0].amount.amount, calculate_net_amount(2000).to_string());

        // Get user's token balance before selling
        let token_balance_before = bank
            .query_balance(&QueryBalanceRequest {
                address: user1.address().to_string(),
                denom: market.token_a.denom.clone(), // "Yes" token
            })
            .unwrap()
            .balance
            .unwrap()
            .amount;

        assert_eq!(token_balance_before, calculate_net_amount(2000).to_string());

        // User1 sells half of their shares (1000 tokens)
        let sell_res = wasm
            .execute(
                &market_address,
                &ExecuteMsg::SellShare {
                    option: "Yes".to_string(),
                },
                &[coin(1000, &market.token_a.denom)], // Send tokens to sell
                &user1,
            )
            .unwrap();

        // Verify the transaction was successful - check for any event indicating success
        assert!(sell_res.events.len() > 0, "No events found in sell response");

        // Check user's shares after selling
        let shares_after: AllSharesResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetShares {
                    market_id: "test_market_1".to_string(),
                    user: Addr::unchecked(user1.address()),
                },
            )
            .unwrap();

        assert_eq!(shares_after.shares.len(), 1);
        assert_eq!(shares_after.shares[0].option, "Yes");
        // After selling 1000 tokens from net amount of 1900
        let remaining_shares = calculate_net_amount(2000) - 1000;
        assert_eq!(shares_after.shares[0].amount.amount, remaining_shares.to_string());

        // Check token balance after selling (should be reduced due to burning)
        let token_balance_after = bank
            .query_balance(&QueryBalanceRequest {
                address: user1.address().to_string(),
                denom: market.token_a.denom.clone(),
            })
            .unwrap()
            .balance
            .unwrap()
            .amount;

        assert_eq!(token_balance_after, (calculate_net_amount(2000)-1000).to_string()); // Reduced from 1900 to 900

        // Verify market totals updated correctly
        let total_shares: TotalSharesPerOptionResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetTotalSharesPerOption {
                    market_id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        //one has been taken as a tax
        // here we tax on the net amount after commission during buy and sell
        assert_eq!(total_shares.amount_a.amount, (calculate_net_amount(2000)-calculate_net_amount(1000)).to_string()); // Was 2000, now 1000 after selling
        assert_eq!(total_shares.amount_b.amount, "0"); // No shares for "No"

        // Verify total value decreased
        let total_value: TotalValueResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetTotalValue {
                    market_id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        //one has been taken as a tax and added to the total value
        assert_eq!(total_value.total_value.amount, (calculate_net_amount(2000)-calculate_net_amount(1000)).to_string()); // Was 2000, now 1001
    }

    #[test]
    fn test_sell_more_shares_than_owned_fails() {
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
            setup_registry_and_market(&wasm, &admin);

        // User1 buys shares for "Yes"
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

        // Get market info to find token denom
        let market: MarketResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetMarket {
                    id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        // Try to sell more tokens than owned (should fail)
        let result = wasm.execute(
            &market_address,
            &ExecuteMsg::SellShare {
                option: "Yes".to_string(),
            },
            &[coin(2000, &market.token_a.denom)], // Trying to sell 2000 when only has 1000
            &user1,
        );

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        println!("Error message: {}", error_msg);
        //we get an expected bank message error
        // assert!(error_msg.contains("Insufficient shares to sell"));
    }

    #[test]
    fn test_sell_shares_no_shares_owned_fails() {
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
            setup_registry_and_market(&wasm, &admin);

        // Get market info to find token denom (needed for attempting to sell)
        let market: MarketResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetMarket {
                    id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        // Try to sell shares without owning any (should fail)
        let result = wasm.execute(
            &market_address,
            &ExecuteMsg::SellShare {
                option: "Yes".to_string(),
            },
            &[coin(1000, &market.token_a.denom)],
            &user1,
        );

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        println!("Error message: {}", error_msg);
        //we get an expected bank message error
        // assert!(error_msg.contains("Insufficient shares to sell"));
    }

    #[test]
    fn test_transfer_shares_then_sell_by_recipient() {
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
        let user_a = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let user_b = app
            .init_account(&[
                coin(100_000_000_000_000_000_000u128, FEE_DENOM),
                coin(100_000_000_000_000_000_000u128, BUY_TOKEN),
            ])
            .unwrap();
        let wasm: Wasm<'_, CoreumTestApp> = Wasm::new(&app);
        let bank = Bank::new(&app);

        let (_registry_address, market_address) =
            setup_registry_and_market(&wasm, &admin);

        // User A buys shares for "Yes"
        wasm.execute(
            &market_address,
            &ExecuteMsg::BuyShare {
                market_id: "test_market_1".to_string(),
                option: "Yes".to_string(),
            },
            &[coin(2000, BUY_TOKEN)],
            &user_a,
        )
        .unwrap();

        // Get market info to find token denom
        let market: MarketResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetMarket {
                    id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        // Verify User A has the tokens and shares
        let user_a_token_balance_before = bank
            .query_balance(&QueryBalanceRequest {
                address: user_a.address().to_string(),
                denom: market.token_a.denom.clone(),
            })
            .unwrap()
            .balance
            .unwrap()
            .amount;

        let user_a_shares_before: AllSharesResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetShares {
                    market_id: "test_market_1".to_string(),
                    user: Addr::unchecked(user_a.address()),
                },
            )
            .unwrap();

        assert_eq!(user_a_token_balance_before, calculate_net_amount(2000).to_string());
        assert_eq!(user_a_shares_before.shares.len(), 1);
        assert_eq!(user_a_shares_before.shares[0].amount.amount, calculate_net_amount(2000).to_string());

        // User A transfers 1500 tokens to User B using bank send
        bank.send(
            MsgSend {
                from_address: user_a.address().to_string(),
                to_address: user_b.address().to_string(),
                amount: vec![coreum_wasm_sdk::types::cosmos::base::v1beta1::Coin {
                    denom: market.token_a.denom.clone(),
                    amount: "1500".to_string(),
                }],
            },
            &user_a,
        )
        .unwrap();

        // Verify User B received the tokens
        let user_b_token_balance_after_transfer = bank
            .query_balance(&QueryBalanceRequest {
                address: user_b.address().to_string(),
                denom: market.token_a.denom.clone(),
            })
            .unwrap()
            .balance
            .unwrap()
            .amount;

        assert_eq!(user_b_token_balance_after_transfer, "1500");

        // Verify User A has remaining tokens
        let user_a_token_balance_after_transfer = bank
            .query_balance(&QueryBalanceRequest {
                address: user_a.address().to_string(),
                denom: market.token_a.denom.clone(),
            })
            .unwrap()
            .balance
            .unwrap()
            .amount;

        // User A transferred 1500 tokens to User B, so remaining balance is 1900 - 1500 = 400 .(1900 is the net amount after commission)
        assert_eq!(user_a_token_balance_after_transfer, (calculate_net_amount(2000)-1500).to_string());

        // Check that User A still has shares recorded in the market contract
        // (shares tracking is separate from token ownership)
        let user_a_shares_after_transfer: AllSharesResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetShares {
                    market_id: "test_market_1".to_string(),
                    user: Addr::unchecked(user_a.address()),
                },
            )
            .unwrap();

        assert_eq!(user_a_shares_after_transfer.shares.len(), 1);
        assert_eq!(user_a_shares_after_transfer.shares[0].amount.amount, (calculate_net_amount(2000)).to_string()); // Shares tracking unchanged

        // User B (who now has tokens but no recorded shares) tries to sell the tokens
        let result_b = wasm.execute(
            &market_address,
            &ExecuteMsg::SellShare {
                option: "Yes".to_string(),
            },
            &[coin(1000, &market.token_a.denom)], // User B tries to sell 1000 of the 1500 received tokens
            &user_b,
        );

        // This should fail because User B has no recorded shares in the market contract
        assert!(result_b.is_err());
        let error_msg_b = result_b.unwrap_err().to_string();
        println!("User B sell error: {}", error_msg_b);
        // User B should get an error because they don't have shares recorded

        // User A can still sell their remaining tokens because they have recorded shares
        let result_a = wasm.execute(
            &market_address,
            &ExecuteMsg::SellShare {
                option: "Yes".to_string(),
            },
            &[coin(400, &market.token_a.denom)], // User A sells their remaining 500 tokens
            &user_a,
        );

        // This should succeed because User A has recorded shares
        assert!(result_a.is_ok());

        // Verify User A's shares were reduced
        let user_a_shares_after_sell: AllSharesResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetShares {
                    market_id: "test_market_1".to_string(),
                    user: Addr::unchecked(user_a.address()),
                },
            )
            .unwrap();

        assert_eq!(user_a_shares_after_sell.shares.len(), 1);
        assert_eq!(user_a_shares_after_sell.shares[0].amount.amount, "1500"); // Reduced from 2000 to 1500

        // Verify User A's token balance is now 0 (sold the 500 they had left)
        let user_a_final_token_balance = bank
            .query_balance(&QueryBalanceRequest {
                address: user_a.address().to_string(),
                denom: market.token_a.denom.clone(),
            })
            .unwrap()
            .balance
            .unwrap()
            .amount;

        assert_eq!(user_a_final_token_balance, "0");

        // User B still has their 1500 tokens but cannot sell them through the market contract
        let user_b_final_token_balance = bank
            .query_balance(&QueryBalanceRequest {
                address: user_b.address().to_string(),
                denom: market.token_a.denom.clone(),
            })
            .unwrap()
            .balance
            .unwrap()
            .amount;

        assert_eq!(user_b_final_token_balance, "1500");
    }

    #[test]
    fn test_time_based_tax_calculation() {
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
        let bank = Bank::new(&app);

        let (_registry_address, market_address) =
            setup_registry_and_market(&wasm, &admin);

        // User1 buys shares
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

        // Get market info
        let market: MarketResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetMarket {
                    id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        // Test selling immediately (should have low tax)
        let sell_res_early = wasm
            .execute(
                &market_address,
                &ExecuteMsg::SellShare {
                    option: "Yes".to_string(),
                },
                &[coin(200, &market.token_a.denom)], // Sell 200 tokens
                &user1,
            )
            .unwrap();

        // Verify early sell has low tax
        let wasm_event = sell_res_early
            .events
            .iter()
            .find(|e| e.ty.starts_with("wasm"))
            .expect("Should find wasm event");
        let tax_rate_early = wasm_event
            .attributes
            .iter()
            .find(|attr| attr.key == "tax_rate")
            .map(|attr| attr.value.clone())
            .unwrap_or_else(|| "0.0".to_string());

        let final_amount_early = sell_res_early
            .events
            .iter()
            .find(|e| e.ty.starts_with("wasm"))
            .and_then(|event| {
                event.attributes
                    .iter()
                    .find(|attr| attr.key == "final_amount")
                    .map(|attr| attr.value.clone())
            })
            .unwrap_or_else(|| "0".to_string());

        let tax_amount_early = sell_res_early
            .events
            .iter()
            .find(|e| e.ty.starts_with("wasm"))
            .and_then(|event| {
                event.attributes
                    .iter()
                    .find(|attr| attr.key == "tax_amount")
                    .map(|attr| attr.value.clone())
            })
            .unwrap_or_else(|| "0".to_string());

        println!("Early sell tax amount: {}", tax_amount_early);
        println!("Early sell tax rate: {}", tax_rate_early);
        println!("Early sell final amount: {}", final_amount_early);

        // Tax rate should be very low early in the market (close to 0)
        let early_tax_rate = Decimal::from_str(&tax_rate_early).unwrap();
        assert!(early_tax_rate < Decimal::from_str("0.1").unwrap()); // Less than 10%

        // Final amount should be close to the original amount (after tax and commission)
        let early_final_amount = Uint128::from_str(&final_amount_early).unwrap();
        println!("Early final amount: {}", early_final_amount);
        // From the debug output: final_amount = 190 (200 - 1 tax - 9 commission)
        assert!(early_final_amount > Uint128::from(180u128)); // Should get back at least 180 out of 200

        // Check user balance increased
        let user_balance_after_early_sell = bank
            .query_balance(&QueryBalanceRequest {
                address: user1.address().to_string(),
                denom: BUY_TOKEN.to_string(),
            })
            .unwrap()
            .balance
            .unwrap()
            .amount;

        println!(
            "User balance after early sell: {}",
            user_balance_after_early_sell
        );

        // User should have received some tokens back
        let initial_balance = Uint128::from_str("100000000000000000000").unwrap();
        let expected_balance = initial_balance -  Uint128::from(1000u128) + Uint128::from_str(&calculate_net_amount(200).to_string()).unwrap() - Uint128::from_str(&tax_amount_early).unwrap(); // 1000 is the amount sold, 500 is the amount bought, tax_amount_early is the tax amount
        assert_eq!(
            Uint128::from_str(&user_balance_after_early_sell).unwrap(),
            //the user only sold 200 on the 100 he bought initially so part of hes inital balance is still there
            expected_balance 
        );
    }

    #[test]
    fn test_sell_after_resolved_market_fails() {
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
            setup_registry_and_market(&wasm, &admin);

        // User1 buys shares
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

        // Resolve the market
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

        // Get market info
        let market: MarketResponse = wasm
            .query(
                &market_address,
                &QueryMsg::GetMarket {
                    id: "test_market_1".to_string(),
                },
            )
            .unwrap();

        // Try to sell after market is resolved (should fail)
        let result = wasm.execute(
            &market_address,
            &ExecuteMsg::SellShare {
                option: "Yes".to_string(),
            },
            &[coin(500, &market.token_a.denom)],
            &user1,
        );

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Cannot sell shares after market is resolved"));
    }

    #[test]
    fn test_get_tax_rate_query() {
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
        let wasm: Wasm<'_, CoreumTestApp> = Wasm::new(&app);

        let (_registry_address, market_address) =
            setup_registry_and_market(&wasm, &admin);

        // Query tax rate at market start (should be close to 0%)
        let tax_rate: TaxRateResponse = wasm
            .query(&market_address, &QueryMsg::GetTaxRate {})
            .unwrap();

        println!("Tax rate at market start: {}", tax_rate.tax_rate);

        assert!(tax_rate.tax_rate < Decimal::from_str("0.1").unwrap());

        // Simulate time passage (advance by half the market duration)
        let block_time = app.get_block_time_seconds() as u64;
        let start_time = get_start_time();
        let end_time = get_end_time();
        let half_duration = (end_time.seconds() - start_time.seconds()) / 2;
        app.increase_time(half_duration as u64);

        // Query tax rate at middle of market (should be around 50%)
        let tax_rate: TaxRateResponse = wasm
            .query(&market_address, &QueryMsg::GetTaxRate {})
            .unwrap();

        println!("Tax rate at middle of market: {}", tax_rate.tax_rate);

        // Should be approximately 50% tax rate
        let expected_rate = Decimal::from_str("0.5").unwrap();
        let tolerance = Decimal::from_str("0.1").unwrap(); // 10% tolerance

        assert!(
            tax_rate.tax_rate >= expected_rate - tolerance
                && tax_rate.tax_rate <= expected_rate + tolerance
        );

        // Advance to near market end
        let near_end_duration = end_time.seconds() - start_time.seconds() - 100; // 100 seconds before end
        app.increase_time(near_end_duration as u64);

        // Query tax rate near market end (should be very high)
        let tax_rate: TaxRateResponse = wasm
            .query(&market_address, &QueryMsg::GetTaxRate {})
            .unwrap();

        println!("Tax rate near market end: {}", tax_rate.tax_rate);

        assert!(tax_rate.tax_rate > Decimal::from_str("0.9").unwrap()); // > 90%
    }

    #[test]
    fn test_simulate_sell_query() {
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
        let wasm: Wasm<'_, CoreumTestApp> = Wasm::new(&app);

        let (_registry_address, market_address) =
            setup_registry_and_market(&wasm, &admin);

        // Simulate sell at market start (should have 0% tax)
        let simulate: SimulateSellResponse = wasm
            .query(
                &market_address,
                &QueryMsg::SimulateSell {
                    option: "Yes".to_string(),
                    amount: "1000".to_string(),
                },
            )
            .unwrap();

        println!("Simulate sell at market start: {:?}", simulate);

        assert_eq!(simulate.amount_sent, "1000");
        assert!(simulate.tax_rate < Decimal::from_str("0.1").unwrap());
        assert_eq!(simulate.tax_amount, "1");
        assert_eq!(simulate.amount_after_tax, "999");

        // Advance time to middle of market
        let block_time = app.get_block_time_seconds() as u64;
        let start_time = get_start_time();
        let end_time = get_end_time();
        let half_duration = (end_time.seconds() - start_time.seconds()) / 2;
        app.increase_time(half_duration as u64);

        // Simulate sell at middle of market (should have ~50% tax)
        let simulate: SimulateSellResponse = wasm
            .query(
                &market_address,
                &QueryMsg::SimulateSell {
                    option: "Yes".to_string(),
                    amount: "1000".to_string(),
                },
            )
            .unwrap();

        println!("Simulate sell at middle of market: {:?}", simulate);

        assert_eq!(simulate.amount_sent, "1000");

        // Tax rate should be around 50%
        let expected_rate = Decimal::from_str("0.5").unwrap();
        let tolerance = Decimal::from_str("0.1").unwrap();
        assert!(
            simulate.tax_rate >= expected_rate - tolerance
                && simulate.tax_rate <= expected_rate + tolerance
        );

        // Tax amount should be around 500 (50% of 1000)
        let tax_amount = Uint128::from_str(&simulate.tax_amount).unwrap();
        assert!(tax_amount >= Uint128::from(400u128) && tax_amount <= Uint128::from(600u128));

        // Amount after tax should be around 500
        let amount_after_tax = Uint128::from_str(&simulate.amount_after_tax).unwrap();
        assert!(
            amount_after_tax >= Uint128::from(400u128)
                && amount_after_tax <= Uint128::from(600u128)
        );

        // Verify calculation consistency
        let calculated_tax = Uint128::from(1000u128) - amount_after_tax;
        assert_eq!(tax_amount, calculated_tax);
    }
}
