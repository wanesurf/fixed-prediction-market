use coreum_wasm_sdk::types::cosmos::base::v1beta1::Coin;
use cosmwasm_std::{Addr, Storage, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
// Define the storage items
// Define the storage items
pub const STATE: Item<State> = Item::new("state");
//Question: is it good pratice to work with references in the storage?
pub const MARKETS: Map<&String, Market> = Map::new("markets");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub admin: Addr,
    pub market_ids: Vec<String>, // Track all market IDs
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Market {
    pub id: String,
    pub options: Vec<String>, // Always two options (binary)
    //TODO: make sure that we track both options/tokens for each users
    pub shares: Vec<Share>,        // Tracks user bets
    pub resolved: bool,            // Whether the market is resolved
    pub outcome: MarketOutcome,    // Unresolved, OptionA, or OptionB
    pub end_time: Timestamp,       // When the market ends
    pub total_value: Coin,         // Total value staked in the market
    pub num_bettors: u64,          // Number of unique bettors
    pub token_a: String,           // Denom for the first option
    pub token_b: String,           // Denom for the second option
    pub buy_token: String,         // Denom for the token used to buy shares
    pub banner_url: String,        // URL for the banner image
    pub description: String,       // Description of the market
    pub title: String,             // Title of the market
    pub end_time_string: String,   // End time of the market
    pub start_time_string: String, // Start time of the market
    pub resolution_source: String, // Source of the resolution
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
//A user can only have 2 shares. Make sure that we increment or decrement on the same share when a user buys or withdraws instead of creating a new share
pub struct Share {
    pub user: Addr,
    pub option: String,
    pub token: Coin,
    pub has_withdrawn: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum MarketOutcome {
    Unresolved,
    OptionA,
    OptionB,
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

impl Market {
    /// Calculate the total stakes for each option
    pub fn total_stakes(&self) -> (Uint128, Uint128) {
        let total_a: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.option == self.options[0])
            .map(|s| Uint128::from_str(&s.token.amount).unwrap_or_default())
            .sum();

        let total_b: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.option == self.options[1])
            .map(|s| Uint128::from_str(&s.token.amount).unwrap_or_default())
            .sum();

        (total_a, total_b)
    }

    pub fn calculate_odds(&self) -> (f64, f64) {
        let total_a: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.option == self.options[0])
            .map(|s| Uint128::from_str(&s.token.amount).unwrap_or_default())
            .sum();

        let total_b: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.option == self.options[1])
            .map(|s| Uint128::from_str(&s.token.amount).unwrap_or_default())
            .sum();

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

        (odds_a, odds_b)
    }

    pub fn calculate_potential_winnings(&self, user: &Addr) -> (Coin, Coin) {
        let (odds_a, odds_b) = self.calculate_odds();

        let user_stake_a: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.user == *user && s.option == self.options[0])
            .map(|s| Uint128::from_str(&s.token.amount).unwrap_or_default())
            .sum();

        let user_stake_b: Uint128 = self
            .shares
            .iter()
            .filter(|s| s.user == *user && s.option == self.options[1])
            .map(|s| Uint128::from_str(&s.token.amount).unwrap_or_default())
            .sum();

        let winnings_a = Coin {
            denom: self.total_value.denom.clone(),
            amount: Uint128::from((user_stake_a.u128() as f64 * odds_a) as u128).to_string(),
        };

        let winnings_b = Coin {
            denom: self.total_value.denom.clone(),
            amount: Uint128::from((user_stake_b.u128() as f64 * odds_b) as u128).to_string(),
        };

        (winnings_a, winnings_b)
    }
    /// Calculate the actual winnings for a user based on the market outcome
    pub fn calculate_winnings(&self, user: &Addr) -> Coin {
        let (total_a, total_b) = self.total_stakes();

        match self.outcome {
            MarketOutcome::OptionA => {
                // User's stake in Option A
                let user_stake_a: Uint128 = self
                    .shares
                    .iter()
                    .filter(|s| s.user == *user && s.option == self.options[0])
                    .map(|s| Uint128::from_str(&s.token.amount).unwrap_or_default())
                    .sum();

                // Calculate winnings: (user_stake_a / total_a) * total_b
                if total_a.is_zero() {
                    return Coin {
                        denom: self.total_value.denom.clone(),
                        amount: "0".to_string(),
                    };
                }

                let winnings = user_stake_a * total_b / total_a;
                Coin {
                    denom: self.total_value.denom.clone(),
                    amount: winnings.to_string(),
                }
            }
            MarketOutcome::OptionB => {
                // User's stake in Option B
                let user_stake_b: Uint128 = self
                    .shares
                    .iter()
                    .filter(|s| s.user == *user && s.option == self.options[1])
                    .map(|s| Uint128::from_str(&s.token.amount).unwrap_or_default())
                    .sum();

                // Calculate winnings: (user_stake_b / total_b) * total_a
                if total_b.is_zero() {
                    return Coin {
                        denom: self.total_value.denom.clone(),
                        amount: "0".to_string(),
                    };
                }

                let winnings = user_stake_b * total_a / total_b;
                Coin {
                    denom: self.total_value.denom.clone(),
                    amount: winnings.to_string(),
                }
            }
            MarketOutcome::Unresolved => {
                // No winnings if the market is unresolved
                Coin {
                    denom: self.total_value.denom.clone(),
                    amount: "0".to_string(),
                }
            }
        }
    }
}
