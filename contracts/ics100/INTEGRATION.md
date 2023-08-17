# This document contains interfaces to interact with ics100 contracts

## Execution Messages
1. Function: `MakeSwap`
    ```
    {
        source_port: <source-port>,
        source_channel: <source-channel>,
        sell_token: <token-to-be-sold>, // Type is `Coin`
        buy_token: <token-to-be-bought>, //Type is `Coin`
        maker_address: <maker-address-from-source-chain>,
        maker_receiving_address: <maker-address-on-destination-chain>,
        desired_taker: <desired-taker>, // if desired_taker is specified, only the desired_taker is allowed to take this order
        timeout_height: <specify> or put Height {revision_number: 0,revision_height: 0},
        timeout_timestamp: <timeout-timestamp>,
        expiration_timestamp: <expiration-timestamp> // In unix time
    }
    ```

2. Function: `TakeSwap`
    ```
    {
        sell_token: <token-to-be-sold> // Type is Coin,
        taker_address: <the sender address>
        taker_receiving_address: <the sender's address on the destination chain>
        timeout_height: <specify> or put Height {revision_number: 0,revision_height: 0},
        timeout_timestamp: <timeout-timestamp>,
    }
    ```

3. Function: `CancelSwap`
    ```
    {
        order_id: <order-id>,
        maker_address: <the sender address>,
        timeout_height: <specify> or put Height {revision_number: 0,revision_height: 0},
        timeout_timestamp: <timeout-timestamp>,
    }
    ```