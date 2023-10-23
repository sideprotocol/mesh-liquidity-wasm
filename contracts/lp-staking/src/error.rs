use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Never {}

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Send some coins to create an atomic swap")]
    EmptyBalance {},

    #[error("Atomic swap not yet expired")]
    NotExpired,

    #[error("Expired atomic swap")]
    Expired,

    #[error("Atomic swap already exists")]
    AlreadyExists,

    #[error("Order already taken")]
    OrderTaken,

    #[error("Order is not for this chain")]
    InvalidChain,

    #[error("Invalid sell token")]
    InvalidSellToken,

    #[error("Order has already been taken")]
    AlreadyTakenOrder,

    #[error("Invalid taker address")]
    InvalidTakerAddress,

    #[error("Invalid maker address")]
    InvalidMakerAddress,

    #[error("Invalid sender address")]
    InvalidSender,

    #[error("Invalid status")]
    InvalidStatus,

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Only supports channel with ibc version ics100-1, got {version}")]
    InvalidIbcVersion { version: String },

    #[error("Only supports unordered channel")]
    OnlyOrderedChannel {},

    #[error("Only accepts tokens that originate on this chain, not native tokens of remote chain")]
    NoForeignTokens {},

    #[error("Parsed port from denom ({port}) doesn't match packet")]
    FromOtherPort { port: String },

    #[error("Parsed channel from denom ({channel}) doesn't match packet")]
    FromOtherChannel { channel: String },

    #[error("Bid is not allowed for this order")]
    TakeBidNotAllowed,

    #[error("Bid already exist")]
    BidAlreadyExist,

    #[error("Bid doesn't exist")]
    BidDoesntExist,
}
