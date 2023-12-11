# LSD contract for SIDE token

## Overview

LSD will provide the layer of smart contracts between delegator and validator which auto-compounds the rewards after a fixed interval without requirement of delegator intervention and provide a derivative sdSIDE which can be used in DeFi.
This uses accuring rewards model i.e all the rewards are accumulated in sdSIDE itself, and sdSIDE value increases over time. Increase in value depends on staking reward rate of chain.

## Features

- Ability to convert staked SIDE into sdSIDE is done by issuing a staking derivative token which represents the user’s delegator’s stake and rewards accumulated.
- This staking derivative token can be swapped with sdSIDE using a DEX supporting such pairs or used in lending.

## Stake-Unstake flow

- User stakes with SIDE and receives sdSIDE in exchange.
- Manager or User calls `claim_and_stake`.(This will claim rewards from validators and restake them).
- User unstakes (sends sdSIDE in contract and gets SIDE coins).
- Manager calls Advance Window(undelegation from validators start here).
- User claims the unbonded amount (after undelegation is completed).

## Exchange rate

- Exchange_rate function calculates

    - total sdSIDE tokens in the supply and
    - total on chain SIDE balance along with rewards (deposited SIDE in contract + rewards accumulated from validators)
    - returns ratio of total sdSIDE to total on chain SIDE.

## Stake from Contract to the validators - Claim_and_stake call

- After users deposit in the contract, Contract manager or user calls ClaimAndStake which triggers claim_and_stake function.
- `claim_and_stake` function collects the rewards accumulated to the validators and
new deposits in the contract and stake the collected amount to the validators.
- `claim_and_stake` is called every x hours to delegate the accumulated rewards and this way users get rid of staking their rewards again and again.
- Staking is done in such a way that keeps difference between staked amount on each validators minimum.

## Unbonding

- When user sends sdSIDE tokens to withdraw, Receive call automatically gets triggered which triggers `try_withdraw` function.

To keep track of undelegating SIDE from validators we are using a data structure named `window_manager`. The schematic is as follows :

```
`window_manager`
      |-- queue_window
      |-- ongoing_window (It is a vecDeque of type ongoingWithWithdrawWindows)

  `queue_window`
      |-- total_sdSIDE amount (total sdSIDE in the queue for undelegation)
      |-- sdSIDE_users_amount (hashmap which contains sdSIDE amount per user)

  `ongoingWithWithdrawWindows`
      |-- total_sdSIDE amount (total sdSIDE)
      |-- total_SIDE amount (corrosponding total SIDE for undelegation)
      |-- side_users_amount (hashmap which contains SIDE corrosponding each user)
```


- On receiving sdSIDE tokens, `try_withdraw` function only updates queue_window's data of window_manager.
- Now, to unbond SIDE from validators, contract manager advances the window and call `AdvanceWindow` which triggers `advance_window_1` function.