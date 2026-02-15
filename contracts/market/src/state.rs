use coreum_wasm_sdk::types::cosmos::base::v1beta1::Coin;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use std::str::FromStr;

use crate::msg::MarketType;

#[cw_serde]
pub struct State {
    pub admin: Addr,
    pub market_ids: Vec<String>, // Track all market IDs
    pub market_id_counter: u64,  // Track the next market ID
    pub last_market_id: u64,     // Track the last market ID
}

#[cw_serde]
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
#[cw_serde]
pub struct Config {
    pub id: String,
    pub admin: Addr,
    pub commission_rate: Uint128, // in basis points (BPS), e.g., 500 = 5%
    pub pairs: Vec<MarketOption>, //represent the options and the tokens --> rename to options
    pub buy_token: String,        // Denom for the token used to buy shares
    pub banner_url: String,       // URL for the banner image
    pub description: String,      // Description of the market
    pub title: String,            // Title of the market
    pub start_time: Timestamp,    // Start time of the market
    pub end_time: Timestamp,      // End time of the market
    pub oracle: Addr,
    pub resolution_source: String, // Source of the resolution --> Feed contract address
    pub asset_to_track: String, //This is the asset name "CORE", "BTC", "ETH", etc. not the DENOM
    pub market_type: MarketType,
    pub target_price: Decimal,
    pub initial_price: Decimal, // Initial price at market creation
}

#[cw_serde]
pub struct MarketState {
    pub status: MarketStatus, // Combined status and outcome
    pub total_value: Coin,
    pub num_bettors: u64,              // Number of unique bettors
    pub total_stake_option_a: Uint128, // Pre-calculated total for option A
    pub total_stake_option_b: Uint128, // Pre-calculated total for option B
    pub volume: Uint128,               // Total volume of the market
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const MARKET_STATE: Item<MarketState> = Item::new("market_state");

// Map with composite key: (user_address, option_text) -> Share
// This allows O(1) lookups and efficient queries
pub const SHARES: Map<(&Addr, &str), Share> = Map::new("shares");

#[cw_serde]
pub struct Share {
    pub amount: Uint128, // Amount of tokens held for this option
    pub has_withdrawn: bool,
}

#[cw_serde]
pub struct MarketStatsResponse {
    pub total_value: Coin,
    pub num_bettors: u64,
    pub odds_a: f64,
    pub odds_b: f64,
}

#[cw_serde]
pub struct UserWinningsResponse {
    pub winnings: Coin,
}

impl MarketState {
    /// Calculate the total stakes for each option (now uses pre-calculated values)
    pub fn total_stakes(&self, _config: &Config) -> (Uint128, Uint128) {
        (self.total_stake_option_a, self.total_stake_option_b)
    }

    pub fn calculate_odds(&self, _config: &Config) -> (Decimal, Decimal) {
        let total_a = self.total_stake_option_a;
        let total_b = self.total_stake_option_b;

        let odds_a = if total_a.is_zero() {
            Decimal::zero()
        } else {
            Decimal::from_ratio(total_b, total_a)
        };

        let odds_b = if total_b.is_zero() {
            Decimal::zero()
        } else {
            Decimal::from_ratio(total_a, total_b)
        };

        (odds_a, odds_b)
    }

    /// Create type-safe odds using market type to ensure correct option assignment
    pub fn create_type_safe_odds(&self, config: &Config) -> Vec<crate::msg::OptionOdds> {
        let (odds_a, odds_b) = self.calculate_odds(config);
        config.market_type.create_option_odds(odds_a, odds_b)
    }

    pub fn calculate_potential_winnings(
        &self,
        storage: &dyn Storage,
        user: &Addr,
        config: &Config,
    ) -> StdResult<(Coin, Coin)> {
        let (odds_a, odds_b) = self.calculate_odds(config);

        // Load user stakes from Map - O(1) lookups
        let user_stake_a = SHARES
            .may_load(storage, (user, &config.pairs[0].text))?
            .map(|s| s.amount)
            .unwrap_or_default();

        let user_stake_b = SHARES
            .may_load(storage, (user, &config.pairs[1].text))?
            .map(|s| s.amount)
            .unwrap_or_default();

        // Commission is now taken during buy/sell operations, so no need to apply it here
        let user_stake_a_after_commission = Decimal::from_str(&user_stake_a.to_string())
            .unwrap_or_default();

        let user_stake_b_after_commission = Decimal::from_str(&user_stake_b.to_string())
            .unwrap_or_default();

        let winnings_a = Decimal::from_str(&user_stake_a_after_commission.to_string())
            .unwrap_or_default()
            * Decimal::from_str(&odds_a.to_string()).unwrap_or_default();

        let winnings_b = Decimal::from_str(&user_stake_b_after_commission.to_string())
            .unwrap_or_default()
            * Decimal::from_str(&odds_b.to_string()).unwrap_or_default();

        let winnings_a = Coin {
            denom: config.buy_token.clone(),
            amount: (winnings_a + user_stake_a_after_commission).to_string(),
        };

        let winnings_b = Coin {
            denom: config.buy_token.clone(),
            amount: (winnings_b + user_stake_b_after_commission).to_string(),
        };

        Ok((winnings_a, winnings_b))
    }

    /// Calculate the actual winnings for a user based on the market outcome
    pub fn calculate_winnings(
        &self,
        storage: &dyn Storage,
        user: &Addr,
        config: &Config,
    ) -> StdResult<Coin> {
        let (winnings_a, winnings_b) = self.calculate_potential_winnings(storage, user, config)?;

        match &self.status {
            MarketStatus::Resolved(winning_option) => {
                if winning_option.text == config.pairs[0].text {
                    Ok(winnings_a)
                } else if winning_option.text == config.pairs[1].text {
                    Ok(winnings_b)
                } else {
                    Ok(Coin {
                        denom: config.buy_token.clone(),
                        amount: "0".to_string(),
                    })
                }
            }
            _ => Ok(Coin {
                denom: self.total_value.denom.clone(),
                amount: "0".to_string(),
            }),
        }
    }

    /// Calculate time-based tax percentage for selling shares
    /// Formula: market_length_in_sec - (market_length_in_sec - 1 * elapsed_time)
    /// Returns a tax percentage between 0.0 and 1.0
    /// The closer to market end, the higher the tax
    pub fn calculate_time_based_tax(&self, config: &Config, current_time: Timestamp) -> Decimal {
        // Calculate total market duration in seconds
        let market_length_sec = config.end_time.seconds() - config.start_time.seconds();

        // If market length is 0 or invalid, return 0 tax
        if market_length_sec == 0 {
            return Decimal::zero();
        }

        // Calculate elapsed time from start
        let elapsed_sec = if current_time.seconds() > config.start_time.seconds() {
            current_time.seconds() - config.start_time.seconds()
        } else {
            0 // Market hasn't started yet
        };

        // If current time is beyond market end, use full market length
        let elapsed_sec = elapsed_sec.min(market_length_sec);

        // Calculate tax using the formula: market_length - (market_length - elapsed)
        // This simplifies to: elapsed_sec / market_length_sec
        // But following your specific formula structure:
        let tax_factor = market_length_sec - (market_length_sec - elapsed_sec);

        // Convert to percentage (0.0 to 1.0)
        // At market start: tax_factor = 0, so tax = 0%
        // At market end: tax_factor = market_length_sec, so tax = 100%
        let tax_percentage = Decimal::from_ratio(tax_factor, market_length_sec);

        // Cap at maximum 100% tax
        tax_percentage.min(Decimal::one())
    }

    /// Calculate the sell amount after applying time-based tax
    pub fn calculate_sell_amount_with_tax(
        &self,
        config: &Config,
        sell_amount: Uint128,
        current_time: Timestamp,
    ) -> Uint128 {
        let tax_rate = self.calculate_time_based_tax(config, current_time);

        // Calculate the retention ratio (1 - tax_rate)
        let amount_after_tax_ratio = Decimal::one() - tax_rate;

        // Convert to decimal, multiply, then convert back
        let sell_amount_decimal = Decimal::from_atomics(sell_amount, 0).unwrap_or_default();
        let amount_after_tax_decimal = sell_amount_decimal * amount_after_tax_ratio;

        // Convert back to Uint128
        amount_after_tax_decimal.to_uint_floor()
    }
}
