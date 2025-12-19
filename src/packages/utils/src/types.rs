use cosmwasm_schema::cw_serde;

use crate::traits::NormalizedName;

#[cw_serde]
pub struct TokenInfo {
    pub denom: String,
    pub decimals: u8,
}

pub type AssetName = String;

impl NormalizedName<AssetName> for AssetName {
    fn normalized(&self) -> String {
        self.to_uppercase()
    }
}

#[cw_serde]
pub struct AssetInfo {
    pub name: AssetName,
    pub denom: String,
    pub decimals: u8,
}
