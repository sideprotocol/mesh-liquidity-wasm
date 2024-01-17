use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};
use cw20::{BalanceResponse, Cw20ReceiveMsg, TokenInfoResponse};
use crate::state::Point;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub guardian_addr: Option<String>,
    pub deposit_token: String,
}

/// This structure describes the execute functions in the contract.
#[cw_serde]
pub enum ExecuteMsg {
    /// Extend the lockup time for your staked LP
    ExtendLockTime { time: u64 },
    /// Receives a message of type [`Cw20ReceiveMsg`] and processes it depending on the received
    /// template.
    Receive(Cw20ReceiveMsg),
    /// Withdraw LP from the contract
    Withdraw {},
}

/// This structure describes a CW20 hook message.
#[cw_serde]
pub enum Cw20HookMsg {
    /// Create a veSIDE position and lock LP for `time` amount of time
    CreateLock { time: u64 },
    /// Deposit veSIDE in another user's LP position
    DepositFor { user: String },
    /// Add more veSIDE to your LP position
    ExtendLockAmount {},
}

// First category
// Calls will be made to lp-token contract

// Send {
//     contract: side1zn9r7u0rgxnwwh08pv2x9l2ut7fz6ya22remklstyfgunq9mhd2qktttm2,
//     amount: "100",
//     msg: Binary(CreateLock {time: <days, weeks, etc in seconds>})
// }

// Send {
//     contract: side1zn9r7u0rgxnwwh08pv2x9l2ut7fz6ya22remklstyfgunq9mhd2qktttm2,
//     amount: "100",
//     msg: Binary(ExtendLockAmount {})
// }

// Second category
// Calls will be made to ve-token contract

// ExtendLockTime { time: u64 },

// Withdraw {},

/// This structure describes the query messages available in the contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Return the user's veSIDE balance
    #[returns(BalanceResponse)]
    Balance { address: String },
    /// Fetch the veSIDE token information
    #[returns(TokenInfoResponse)]
    TokenInfo {},
    /// Return the current total amount of veSIDE
    #[returns(VotingPowerResponse)]
    TotalVotingPower {},
    /// Return the total amount of veSIDE at some point in the past
    #[returns(VotingPowerResponse)]
    TotalVotingPowerAt { time: u64 },
    /// Return the total voting power at a specific period
    #[returns(VotingPowerResponse)]
    TotalVotingPowerAtPeriod { period: u64 },
    /// Return the user's current voting power (veSIDE balance)
    #[returns(VotingPowerResponse)]
    UserVotingPower { user: String },
    /// Return the user's veSIDE balance at some point in the past
    #[returns(VotingPowerResponse)]
    UserVotingPowerAt { user: String, time: u64 },
    /// Return the user's voting power at a specific period
    #[returns(VotingPowerResponse)]
    UserVotingPowerAtPeriod { user: String, period: u64 },
    /// Return information about a user's lock position
    #[returns(LockInfoResponse)]
    LockInfo { user: String },
    /// Return user's locked LP balance at the given block height
    #[returns(Uint128)]
    UserDepositAtHeight { user: String, height: u64 },
    /// Return the  veSIDE contract configuration
    #[returns(ConfigResponse)]
    Config {},
    /// Return the veSIDE amount for staking x amount of lp-token or adding some time
    #[returns(Point)]
    SimulateLock {
        user: String,
        add_amount: Option<Uint128>,
        time: Option<u64>, 
    },
}

/// This structure is used to return a user's amount of veSIDE.
#[cw_serde]
pub struct VotingPowerResponse {
    /// The veSIDE balance
    pub voting_power: Uint128,
}

/// This structure is used to return the lock information for a veSIDE position.
#[cw_serde]
pub struct LockInfoResponse {
    /// The amount of LP locked in the position
    pub amount: Uint128,
    /// This is the initial boost for the lock position
    pub coefficient: Decimal,
    /// Start time for the veSIDE position decay
    pub start: u64,
    /// End time for the veSIDE position decay
    pub end: u64,
    /// Slope at which a staker's veSIDE balance decreases over time
    pub slope: Uint128,
}

/// This structure stores the parameters returned when querying for a contract's configuration.
#[cw_serde]
pub struct ConfigResponse {
    pub admin: String,
    pub deposit_token: String,
}

/// This structure describes a Migration message.
#[cw_serde]
pub struct MigrateMsg {}
