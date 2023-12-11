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
    ```rust
    let total_on_chain = get_onchain_balance_with_rewards(querier, store, &contract_address)?;
    let tokens = query_total_supply(querier, &sdSIDE_token)?
        .u128()
        .saturating_sub(state.sdSIDE_to_burn.u128());
    let exchange_rate = _calc_exchange_rate(total_on_chain, tokens)?;
    Ok(exchange_rate)
    ```

## Stake from Contract to the validators - Claim_and_stake call

- After users deposit in the contract, Contract manager or user calls ClaimAndStake which triggers claim_and_stake function.
- `claim_and_stake` function collects the rewards accumulated to the validators and
new deposits in the contract and stake the collected amount to the validators.
- `claim_and_stake` is called every x hours to delegate the accumulated rewards and this way users get rid of staking their rewards again and again.
- Staking is done in such a way that keeps difference between staked amount on each validators minimum.
```rust
let slashing_amount = (state.sdSIDE_backing)
        .saturating_sub(state.to_deposit + Uint128::from(validator_set.total_staked()));
state.sdSIDE_backing = state
    .sdSIDE_backing
    .saturating_sub(Uint128::from(slashing_amount.u128()));

// claim rewards
messages.append(&mut validator_set.withdraw_rewards_messages());

let reward_amount =
    get_rewards(deps.storage, deps.querier, &env.contract.address).unwrap_or_default();

let fee = calc_fee(reward_amount, config.dev_fee);
```

## Unbonding

- When user sends sdSIDE tokens to withdraw, Receive call automatically gets triggered which triggers `try_withdraw` function.

To keep track of unbonding SIDE from validators we are using a data structure named `window_manager`. The schematic is as follows :

```rust
`window_manager`
      |-- queue_window
      |-- ongoing_window (It is a vecDeque of type ongoingWithWithdrawWindows)

  `queue_window`
      |-- total_sdSIDE amount (total sdSIDE in the queue for unbonding)
      |-- sdSIDE_users_amount (hashmap which contains sdSIDE amount per user)

  `ongoingWithWithdrawWindows`
      |-- total_sdSIDE amount (total sdSIDE)
      |-- total_SIDE amount (corresponding total SIDE for unbonding)
      |-- side_users_amount (hashmap which contains SIDE corresponding each user)
```


- On receiving sdSIDE tokens, `try_withdraw` function only updates queue_window's data of window_manager.
- Now, to unbond SIDE from validators, contract manager advances the window and call `AdvanceWindow` which triggers `advance_window_1` function.
```rust
let mut validator_set = VALIDATOR_SET.load(deps.storage)?;
let mut window_manager = WINDOW_MANANGER.load(deps.storage)?;

// Store user's sdSIDE amount in active window (WithdrawWindow)
window_manager.add_user_amount_to_active_window(
    deps.storage,
    Addr::unchecked(_cw20_msg.sender.clone()),
    _cw20_msg.amount,
)?;
let total_sdside_amount = window_manager.queue_window.total_sdSIDE;
let user_sdside_amount = window_manager.get_user_sdSIDE_in_active_window(
    deps.storage,
    Addr::unchecked(_cw20_msg.sender.clone()),
    )?;

```

## Batched Unbonding Requests
- Due to a limit on simultaneous unbonding requests per account, unbonding requests are batched to ensure efficient processing.
- A maximum of 7 simultaneous unbonding requests is allowed for a single account.
- Unbonding requests are accumulated until a batch is formed.
Batch time is calculated as (unbonding period) / 7 to comply with the simultaneous unbonding request limit.
