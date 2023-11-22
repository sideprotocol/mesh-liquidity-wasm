use serde::{Deserialize, Serialize};

use cosmwasm_std::StdError;
use std::convert::TryFrom;

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq)]
pub enum KillSwitch {
    Closed,
    Unbonding,
    Open,
}

impl TryFrom<u8> for KillSwitch {
    type Error = StdError;

    fn try_from(other: u8) -> Result<Self, Self::Error> {
        match other {
            0 => Ok(Self::Closed),
            1 => Ok(Self::Unbonding),
            2 => Ok(Self::Open),
            _ => Err(StdError::generic_err("Failed to convert killswitch enum")),
        }
    }
}

impl Into<u8> for KillSwitch {
    fn into(self) -> u8 {
        match self {
            Self::Closed => 0u8,
            Self::Unbonding => 1u8,
            Self::Open => 2u8,
        }
    }
}
