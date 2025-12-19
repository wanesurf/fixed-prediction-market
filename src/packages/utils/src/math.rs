use cosmwasm_std::{Decimal, StdError, Uint128};

pub fn mul_128_by_decimal(a: Uint128, b: Decimal) -> Result<Uint128, StdError> {
    let a_decimal = Decimal::from_atomics(a, 0)
        .map_err(|_| StdError::generic_err("Overflow when converting Uint128 to Decimal"))?;
    let result = a_decimal * b;
    let result_uint = Uint128::try_from(result.to_uint_floor())
        .map_err(|_| StdError::generic_err("Error when converting Decimal to Uint128"))?;

    Ok(result_uint)
}

pub fn div_128_by_decimal(a: Uint128, b: Decimal) -> Result<Decimal, StdError> {
    let a_decimal = Decimal::from_atomics(a, 0)
        .map_err(|_| StdError::generic_err("Overflow when converting Uint128 to Decimal"))?;
    let result = a_decimal / b;

    Ok(result)
}
