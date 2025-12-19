use crate::error::ContractError;
use cosmwasm_std::{Addr, MessageInfo, Uint128};

/// Validates that a string is a valid address
pub fn validate_address(addr: &str) -> Result<Addr, ContractError> {
    // TODO: implement this properly
    Ok(Addr::unchecked(addr))
}

/// Validates that funds match expected denom and amount
pub fn validate_funds(
    info: &MessageInfo,
    expected_denom: &str,
    expected_amount: Option<Uint128>,
) -> Result<Uint128, ContractError> {
    let extracted = extract_denom_funds(info, expected_denom)?;
    
    if let Some(expected) = expected_amount {
        if extracted < expected {
            return Err(ContractError::InvalidAmount {
                amount: extracted,
            });
        }
    }

    Ok(extracted)
}

/// Check if funds match expected amount and denom
pub fn extract_denom_funds(
    info: &MessageInfo,
    expected_denom: &str,
) -> Result<Uint128, ContractError> {
    if info.funds.is_empty() {
        return Err(ContractError::NoFunds {});
    }

    let coin = info.funds.clone()
        .into_iter()
        .find(|coin| coin.denom == expected_denom)
        .ok_or(ContractError::DenomNotFound { denom: expected_denom.to_string() })?;

    if coin.amount.is_zero() {
        return Err(ContractError::InvalidAmount {
            amount: Uint128::zero(),
        });
    }

    let amount_128 = Uint128::try_from(coin.amount).map_err(|_| {
        ContractError::Std(cosmwasm_std::StdError::generic_err("Invalid conversion"))
    })?;

    Ok(amount_128)
}