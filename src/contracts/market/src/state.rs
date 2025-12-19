use coreum_wasm_sdk::types::cosmos::base::v1beta1::Coin;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, StdError, Storage, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::ContractError;
// Define the storage items
// Define the storage items
//Question: is it good pratice to work with references in the storage?

pub const COMMISSION_RATE: Decimal = Decimal::percent(5);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub admin: Addr,
    pub market_ids: Vec<String>, // Track all market IDs
    pub market_id_counter: u64,  // Track the next market ID
    pub last_market_id: u64,     // Track the last market ID
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketOption {
    pub text: String, // The display text (e.g., "YES", "NO", "Trump", "Biden")
    pub associated_token_denom: String,
}

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// pub struct TokenInfo {
//     pub option: MarketOption,
//     pub denom: String,
// }

#[cw_serde]
pub enum MarketStatus {
    Pending,                // Market created but not yet active
    Active,                 // Market is open for betting
    Closed,                 // Market has ended, no more bets, awaiting resolution
    Resolved(MarketOption), // Market resolved with winning option
    Cancelled,              // Market was cancelled
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub id: String,
    pub registry_address: Addr,
    pub pairs: Vec<MarketOption>,  //represent the options and the tokens
    pub end_time: String,          // When the market ends
    pub buy_token: String,         // Denom for the token used to buy shares
    pub banner_url: String,        // URL for the banner image
    pub description: String,       // Description of the market
    pub title: String,             // Title of the market
    pub end_time_string: String,   // End time of the market
    pub start_time_string: String, // Start time of the market
    pub resolution_source: String, // Source of the resolution --> Feed contract address
                                   // pub liquidity: String,         // Liquidity of the market
                                   //odds: <Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketState {
    pub shares: Vec<Share>,
    pub status: MarketStatus, // Combined status and outcome
    pub total_value: Coin,
    pub num_bettors: u64, // Number of unique bettors
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const MARKET_STATE: Item<MarketState> = Item::new("market_state");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
//A user can only have 2 shares.We increment or decrement on the same share when a user buys or withdraws instead of creating a new share
// We use the option to reference which market option this share is for
pub struct Share {
    pub user: Addr,
    pub option: MarketOption, // References which option from Config.pairs
    pub amount: Uint128,      // Amount of tokens held for this option
    pub has_withdrawn: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketStatsResponse {
    pub total_value: Coin,
    pub num_bettors: u64,
    pub odds_a: f64,
    pub odds_b: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserWinningsResponse {
    pub winnings: Coin,
}

impl MarketState {
    /// Calculate the total stakes for each option
    pub fn total_stakes(&self, config: &Config) -> (Uint128, Uint128) {
        let total_a: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.option.text == config.pairs[0].text)
            .map(|s| s.amount)
            .sum();

        let total_b: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.option.text == config.pairs[1].text)
            .map(|s| s.amount)
            .sum();

        (total_a, total_b)
    }

    pub fn calculate_odds(&self, config: &Config) -> (Decimal, Decimal) {
        let total_a: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.option.text == config.pairs[0].text)
            .map(|s| s.amount)
            .sum();

        let total_b: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.option.text == config.pairs[1].text)
            .map(|s| s.amount)
            .sum();

        let odds_a = if total_a.is_zero() {
            Decimal::zero()
        } else {
            Decimal::from_ratio(total_b, total_a)
        };

        let odds_b = if total_b.is_zero() {
            Decimal::zero()
        } else {
            Decimal::from_ratio(total_a, total_b)
            // .checked_mul(Decimal::one() - COMMISSION_RATE)
            // .unwrap_or_default()
        };

        (odds_a, odds_b)
    }

    pub fn calculate_potential_winnings(&self, user: &Addr, config: &Config) -> (Coin, Coin) {
        let (odds_a, odds_b) = self.calculate_odds(config);

        let user_stake_a: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.user == *user && s.option.text == config.pairs[0].text)
            .map(|s| s.amount)
            .sum();

        let user_stake_a_after_commission = Decimal::from_str(&user_stake_a.to_string())
            .unwrap_or_default()
            * (Decimal::one() - COMMISSION_RATE);

        let user_stake_b: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.user == *user && s.option.text == config.pairs[1].text)
            .map(|s| s.amount)
            .sum();

        let user_stake_b_after_commission = Decimal::from_str(&user_stake_b.to_string())
            .unwrap_or_default()
            * Decimal::from_str(&(Decimal::one() - COMMISSION_RATE).to_string())
                .unwrap_or_default();

        let winnings_a = Decimal::from_str(&user_stake_a_after_commission.to_string())
            .unwrap_or_default()
            * Decimal::from_str(&odds_a.to_string()).unwrap_or_default();

        let winnings_b = Decimal::from_str(&user_stake_b_after_commission.to_string())
            .unwrap_or_default()
            * Decimal::from_str(&odds_b.to_string()).unwrap_or_default();

        //the payout should be in the same token as the buy_token
        let winnings_a = Coin {
            denom: config.buy_token.clone(),
            amount: (winnings_a + user_stake_a_after_commission).to_string(),
        };

        let winnings_b = Coin {
            denom: config.buy_token.clone(),
            amount: (winnings_b + user_stake_b_after_commission).to_string(),
        };

        (winnings_a, winnings_b)
    }
    /// Calculate the actual winnings for a user based on the market outcome
    pub fn calculate_winnings(&self, user: &Addr, config: &Config) -> Coin {
        let (winnings_a, winnings_b) = self.calculate_potential_winnings(user, config);

        //We always compare [0] with token_a and [1] with token_b so it should be ok?

        match &self.status {
            MarketStatus::Resolved(winning_option) => {
                if winning_option.text == config.pairs[0].text {
                    winnings_a
                } else if winning_option.text == config.pairs[1].text {
                    winnings_b
                } else {
                    // This shouldn't happen if the market is properly maintained
                    Coin {
                        denom: config.buy_token.clone(),
                        amount: "0".to_string(),
                    }
                }
            }
            // Market not resolved yet - no winnings
            _ => Coin {
                denom: self.total_value.denom.clone(),
                amount: "0".to_string(),
            },
        }
    }
}
