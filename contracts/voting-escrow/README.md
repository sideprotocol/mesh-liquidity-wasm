# Vote-Escrowed SIDE (veSIDE)

# Features

- veSIDE is represented by the VotingEscrow contract.
- veSIDE cannot be transferred. The only way to obtain veSIDE is by locking SIDE-ATOM LP. The maximum lock time is not set yet.
- A user’s veSIDE balance decays linearly as the remaining time until the LP unlock decreases. For example, a balance of 4000 LP locked for one year provides the same amount of veSIDE as 2000 LP locked for two years, or 1000 LP locked for four years.

# Implementation Details

User voting power `w_i` is linearly decreasing since the moment of lock. So does the total voting power `W`. In order to avoid periodic check-ins, every time the user deposits, or withdraws, or changes the locktime, we record user’s slope and bias for the linear function `w_i_t` in the public mapping `user_point_history`. We also change slope and bias for the total voting power `W_t` and record it in `point_history`. In addition, when a user’s lock is scheduled to end, we schedule change of slopes of `W_t`in the future in `slope_changes`. Every change involves increasing the `epoch` by 1.

This way we don’t have to iterate over all users to figure out, how much should 
`W_t`change by, neither we require users to check in periodically. However, we limit the end of user locks to times rounded off by whole weeks.

Slopes and biases change both when a user deposits and locks governance tokens, and when the locktime expires. All the possible expiration times are rounded to whole weeks to make number of reads from blockchain proportional to number of missed weeks at most, not number of users (which is potentially large).

Ref: [CRV](https://curve.readthedocs.io/dao-vecrv.html), [BAL](https://docs.balancer.fi/concepts/governance/veBAL/)

# Querying Balances, Locks and supply

## User balance
```
    /// Return the user's veSIDE balance
    Balance { address: String },
```
`address`: User address

## TokenInfo
```
/// Fetch the veSIDE token information
TokenInfo {},
```

## Total voting power
```
/// Return the current total amount of veSIDE
TotalVotingPower {},
```

## Total voting power
```
/// Return the total amount of veSIDE at some point in the past
TotalVotingPowerAt { time: u64 },
```
`time`: Time in seconds

