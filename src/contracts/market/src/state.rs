use coreum_wasm_sdk::types::cosmos::base::v1beta1::Coin;
use cosmwasm_std::{Addr, Decimal, StdError, Storage, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::ContractError;
// Define the storage items
// Define the storage items
pub const STATE: Item<State> = Item::new("state");
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
//directly couples an option with its corresponding token (Coin
pub struct MarketPair {
    pub option: MarketOption,
    pub token: Coin,
}

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// pub struct TokenInfo {
//     pub option: MarketOption,
//     pub denom: String,
// }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]

pub enum MarketStatus {
    Pending,
    Active,
    Resolved,
    Cancelled,
    Expired,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub id: String,
    pub registry_address: Addr,
    pub pairs: Vec<MarketPair>,    //represent the options and the tokens
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
    pub resolved: bool,
    pub outcome: MarketOutcome,
    pub total_value: Coin,
    pub num_bettors: u64, // Number of unique bettors
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const MARKET_STATE: Item<MarketState> = Item::new("market_state");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
//A user can only have 2 shares.We increment or decrement on the same share when a user buys or withdraws instead of creating a new share
// We use the pair to thight the between the user and the market
pub struct Share {
    pub user: Addr,
    pub pair: MarketPair, //represent the option and the token
    pub has_withdrawn: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum MarketOutcome {
    Unresolved,
    Resolved(MarketOption), // Store the winning option
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
            .filter(|s| s.pair.option.text == config.pairs[0].option.text)
            .map(|s| Uint128::from_str(&s.pair.token.amount).unwrap_or_default())
            .sum();

        let total_b: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.pair.option.text == config.pairs[1].option.text)
            .map(|s| Uint128::from_str(&s.pair.token.amount).unwrap_or_default())
            .sum();

        (total_a, total_b)
    }

    pub fn calculate_odds(&self, config: &Config) -> (Decimal, Decimal) {
        let total_a: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.pair.option.text == config.pairs[0].option.text)
            .map(|s| Uint128::from_str(&s.pair.token.amount).unwrap_or_default())
            .sum();

        let total_b: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.pair.option.text == config.pairs[1].option.text)
            .map(|s| Uint128::from_str(&s.pair.token.amount).unwrap_or_default())
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
            .filter(|s| s.user == *user && s.pair.option.text == config.pairs[0].option.text)
            .map(|s| Uint128::from_str(&s.pair.token.amount).unwrap_or_default())
            .sum();

        let user_stake_a_after_commission = Decimal::from_str(&user_stake_a.to_string())
            .unwrap_or_default()
            * (Decimal::one() - COMMISSION_RATE);

        let user_stake_b: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.user == *user && s.pair.option.text == config.pairs[1].option.text)
            .map(|s| Uint128::from_str(&s.pair.token.amount).unwrap_or_default())
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

        //We always compare [0] wih token_a and [1] with token_b so it should be ok?

        match &self.outcome {
            MarketOutcome::Unresolved => Coin {
                denom: self.total_value.denom.clone(),
                amount: "0".to_string(),
            },
            MarketOutcome::Resolved(winning_option) => {
                if winning_option.text == config.pairs[0].option.text {
                    winnings_a
                } else if winning_option.text == config.pairs[1].option.text {
                    winnings_b
                } else {
                    // This shouldn't happen if the market is properly maintained
                    Coin {
                        denom: config.buy_token.clone(),
                        amount: "0".to_string(),
                    }
                }
            }
        }
    }
}
