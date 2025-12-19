use cosmwasm_std::{instantiate2_address, CanonicalAddr, HexBinary, StdError, StdResult};

pub fn derive_address2(
    canonical_creator: CanonicalAddr,
    salt: &[u8],
    code_hex: &str,
) -> StdResult<CanonicalAddr> {
    let checksum = HexBinary::from_hex(code_hex)?;
    let canonical_addr = instantiate2_address(&checksum, &canonical_creator, salt)
        .map_err(|e| StdError::generic_err(e.to_string()))?;
    Ok(canonical_addr)
}
