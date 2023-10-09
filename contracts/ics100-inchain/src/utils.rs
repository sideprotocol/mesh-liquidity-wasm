use cosmwasm_std::{Addr, BankMsg, Coin, StdError, StdResult, SubMsg};

use sha2::{Digest, Sha256};

pub fn generate_order_id(order_path: &str) -> StdResult<String> {
    // Generate random bytes
    // Create the ID by combining the order_path and random bytes
    let res: Vec<u8> = [order_path.as_bytes()].concat();

    // Hash the result to create a fixed-length ID
    let hash = Sha256::digest(&res);
    let id = hex::encode(hash);

    Ok(id)
}

pub fn order_path(
    source_channel: String,
    source_port: String,
    destination_channel: String,
    destination_port: String,
    sequence: u64,
) -> StdResult<String> {
    let path = format!(
        "channel/{}/port/{}/channel/{}/port/{}/{}",
        source_channel, source_port, destination_channel, destination_port, sequence
    );
    Ok(path)
}

pub fn extract_source_channel_for_taker_msg(path: &str) -> StdResult<String> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 6 {
        return Err(StdError::generic_err("Invalid path"));
    }
    Ok(parts[5].to_string())
}

pub(crate) fn send_tokens(to: &Addr, amount: Coin) -> StdResult<Vec<SubMsg>> {
    let msg = BankMsg::Send {
        to_address: to.into(),
        amount: vec![amount],
    };
    Ok(vec![SubMsg::new(msg)])
}
