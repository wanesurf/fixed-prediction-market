use coreum_wasm_sdk::types::{
    coreum::asset::ft::v1::QueryBalanceResponse, cosmos::base::v1beta1::Coin,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Timestamp, Uint128};

use crate::state::{MarketOption, MarketStatus};

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct InstantiateMsg {
    pub id: String,
    pub admin: Addr,
    pub options: Vec<String>, //outcomes options
    pub start_time: Timestamp,
    pub end_time: Timestamp,
    pub commission_rate: Uint128, // in basis points (BPS), e.g., 500 = 5%
    pub buy_token: String,
    pub banner_url: String,
    pub description: String,
    pub title: String,
    pub resolution_source: String, // NOT FOR NOW 
    // The denom we're tracking on the clp_feed contract
    pub asset_to_track: String, //This is the asset name "CORE", "BTC", "ETH", etc. not the DENOM
    //"up_down" -->  price to beat + duration. example: "Bitcoin Up or Down - 5 min"
    //"price_at" --> "Will Bitcoin be higher then 80k on the 15th of March 2026"
    pub market_type: MarketType,
    // If we reach this price market resolve to UP or YES
    pub target_price: Decimal,
  
    pub oracle: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    BuyShare {
        market_id: String,
        option: String,
    },
    Resolve {
        market_id: String,
    },
    Withdraw {
        market_id: String,
    },
    SellShare {
        option: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(MarketResponse)]
    GetMarket { id: String },
    #[returns(AllSharesResponse)]
    GetShares { market_id: String, user: Addr },
    #[returns(MarketStatsResponse)]
    GetMarketStats { market_id: String }, // Total value, number of bettors, odds
    #[returns(UserPotentialWinningsResponse)]
    GetUserPotentialWinnings { market_id: String, user: Addr }, // User's potential winnings
    #[returns(UserWinningsResponse)]
    GetUserWinnings { market_id: String, user: Addr }, // User's actual winnings
    #[returns(QueryBalanceResponse)]
    GetUserBalance { user: String, denom: String }, // User's balance
    #[returns(AllSharesResponse)]
    GetAllShares { market_id: String }, // All shares
    //Get total stakes: Amount of tokens in the market PER option
    #[returns(TotalValueResponse)]
    GetTotalValue { market_id: String }, // Total stakes
    #[returns(TotalSharesPerOptionResponse)]
    GetTotalSharesPerOption { market_id: String }, // Total shares per option
    #[returns(OddsResponse)] //TOD):rename!
    GetOdds { market_id: String }, // Total shares per option
    #[returns(TaxRateResponse)]
    GetTaxRate {}, // Current tax rate for selling
    #[returns(SimulateSellResponse)]
    SimulateSell {
        option: String,
        amount: String,
    }, // Simulate selling shares
}

// We define a custom struct for each query response

#[cw_serde]
pub struct OptionWithOdds {
    pub option: String,
    pub odds: String,
    pub token_denom: String,
    pub total_staked: String,
}

#[cw_serde]
pub struct MarketResponse {
    pub id: String,
    pub options: Vec<String>,
    pub status: MarketStatus,
    pub total_value: Coin,
    pub num_bettors: u64,
    pub token_a: Coin,
    pub token_b: Coin,
    pub buy_token: String,
    pub banner_url: String,
    pub description: String,
    pub title: String,
    pub end_time: Timestamp,
    pub start_time: Timestamp,
    pub resolution_source: String,
    pub options_with_odds: Vec<OptionWithOdds>,
    // pub liquidity: String
}

#[cw_serde]
pub struct OptionOdds {
    pub option: String,
    pub odds: Decimal,
}

#[cw_serde]
pub struct OddsResponse {
    pub odds: Vec<OptionOdds>,
}

#[cw_serde]
pub struct TotalValueResponse {
    pub total_value: Coin,
}

#[cw_serde]
pub struct OptionShares {
    pub option: String,
    pub token_denom: String,
    pub total_staked: Coin,
}

#[cw_serde]
pub struct TotalSharesPerOptionResponse {
    pub options: Vec<OptionShares>,
    // Legacy fields for backward compatibility
    pub option_a: MarketOption,
    pub amount_a: Coin,
    pub option_b: MarketOption,
    pub amount_b: Coin,
}

#[cw_serde]
pub struct ShareResponse {
    pub user: Addr,
    pub option: String,
    pub amount: Coin,
    pub has_withdrawn: bool, //
}
#[cw_serde]
pub struct AllSharesResponse {
    pub shares: Vec<ShareResponse>,
}
#[cw_serde]
pub struct MarketStatsResponse {
    pub total_value: Coin,
    pub num_bettors: u64,
    pub options_odds: Vec<OptionOdds>,
    // Legacy fields for backward compatibility
    pub odds_a: Decimal,
    pub odds_b: Decimal,
}
#[cw_serde]
pub struct OptionPotentialWinning {
    pub option: String,
    pub potential_winnings: Coin,
}

#[cw_serde]
pub struct UserPotentialWinningsResponse {
    pub options: Vec<OptionPotentialWinning>,
    // Legacy fields for backward compatibility
    pub potential_win_a: Coin,
    pub potential_win_b: Coin,
}
#[cw_serde]
pub struct UserWinningsResponse {
    pub winnings: Coin,
}

#[cw_serde]
pub struct TaxRateResponse {
    pub tax_rate: Decimal, // Current tax rate as a decimal (0.0 to 1.0)
}

#[cw_serde]
pub struct SimulateSellResponse {
    pub amount_sent: String,       // Amount user wants to sell
    pub tax_rate: Decimal,         // Tax rate applied
    pub tax_amount: String,        // Amount taken as tax
    pub amount_after_tax: String,  // Amount user would receive
}
#[cw_serde]
pub enum MarketType {
    UpDown,
    PriceAt,
}

impl std::fmt::Display for MarketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketType::UpDown => write!(f, "UpDown"),
            MarketType::PriceAt => write!(f, "PriceAt"),
        }
    }
}

impl MarketType {
    /// Returns the option text that wins when the condition is met (price target reached)
    pub fn get_winning_option_when_target_reached(&self) -> &'static str {
        match self {
            MarketType::UpDown => "Up",
            MarketType::PriceAt => "Yes",
        }
    }

    /// Returns the option text that wins when the condition is not met (price target not reached)
    pub fn get_winning_option_when_target_not_reached(&self) -> &'static str {
        match self {
            MarketType::UpDown => "Down",
            MarketType::PriceAt => "No",
        }
    }

    /// Determines the winning option based on whether target price was reached
    pub fn determine_winner(&self, current_price: cosmwasm_std::Decimal, target_price: cosmwasm_std::Decimal) -> &'static str {
        if current_price >= target_price {
            self.get_winning_option_when_target_reached()
        } else {
            self.get_winning_option_when_target_not_reached()
        }
    }

    /// Validates that the provided options match the expected options for this market type
    pub fn validate_options(&self, options: &[String]) -> Result<(), String> {
        if options.len() != 2 {
            return Err("Markets must have exactly two options".to_string());
        }

        let expected_options = match self {
            MarketType::UpDown => vec!["Up", "Down"],
            MarketType::PriceAt => vec!["Yes", "No"],
        };

        let mut provided_options = options.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        provided_options.sort();
        let mut expected_sorted = expected_options.clone();
        expected_sorted.sort();

        if provided_options != expected_sorted {
            return Err(format!(
                "Options {:?} do not match expected options {:?} for market type {}",
                options, expected_options, self
            ));
        }

        Ok(())
    }
}
