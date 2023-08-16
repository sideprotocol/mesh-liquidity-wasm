# This document contains interfaces to interact with ics101 contracts

## Execution Messages
1. Make Pool
    - Function: `MakePool`
    ```
      {
        sourcePort: `wasm.${getWasmChainIDContractAddress(chain.chainID)}`,
        sourceChannel: getChannelIdsByChainID(chain.chainID, remoteChain.chainID),
        counterpartyChannel: getChannelIdsByChainID(
            remoteChain.chainID,
            chain.chainID
        ),
        creator: nativeAddress,
        counterpartyCreator: remoteAddress,
        liquidity: [
            {
            side: 'SOURCE',
            balance: {
                denom: poolCreateStore.native.coin.denom,
                amount: getUnitAmount(
                poolCreateStore.native.amount,
                poolCreateStore.native.coin.denom
                ),
            },
            weight: 50,
            decimal: parseInt(chain?.assets?.[0].exponent),
            },
            {
            side: 'DESTINATION',
            balance: {
                denom: poolCreateStore.remote.coin.denom,
                amount: getUnitAmount(
                poolCreateStore.remote.amount,
                poolCreateStore.remote.coin.denom
                ),
            },
            weight: 50,
            decimal: parseInt(remoteChain?.assets?.[0].exponent),
            },
        ],
        swapFee: poolCreateStore.feeRatio * 100,
        timeoutHeight: 100,
        timeoutTimestamp: 100,
        sourceChainId: chain.chainID,
        destinationChainId: remoteChain.chainID,
        }
      ```

2. Take Pool
    - Function: `TakePool`
    ```
      {
        creator: remoteAddress,
        counterCreator: nativeAddress,
        poolId: <poolid-here>,
        timeoutHeight: 100,
        timeoutTimestamp: 100,
      }
      ```

3. Cancel Pool
    - Function: `CancelPool`
    ```
      {
        poolId: <poolid-here>,
        timeoutHeight: 100,
        timeoutTimestamp: 100,
      }
    ```

4. Make Multi Asset Order
    - Function: `MakeMultiAssetDeposit`
    ```
      {
        chainId: chain.chainID,
        poolId: poolItem.id,
        deposits: [sourceAsset, targetAsset], // Here sourceAsset and targetAsset type is DepositAsset
        timeoutHeight: 100,
        timeoutTimestamp: 100,
      }
      ```

      DepositAsset: Example
      ```
        const targetAsset: DepositAsset = {
            sender: remoteAddress, // string
            balance: remoteDepositCoin, // Here remoteDepositCoin type `Coin`
        };
      ```

5. Take Multi Asset Order
    - Function: `TakeMultiAssetDeposit`
    ```
      {
        poolId: poolDepositStore.poolDepositLastOrder?.poolId,
        orderId: poolDepositStore.poolDepositLastOrder?.id,
        sender: poolDepositStore.poolDepositLastOrder?.destinationTaker,
        timeoutHeight: 100,
        timeoutTimestamp: 100,
      }
      ```

6. Cancel Multi Asset Order
    - Function: `CancelMultiAssetDeposit`
    ```
      {
        poolId: poolDepositStore.poolDepositLastOrder?.poolId,
        orderId: poolDepositStore.poolDepositLastOrder?.id,
        sender: poolDepositStore.poolDepositLastOrder?.destinationTaker,
        timeoutHeight: 100,
        timeoutTimestamp: 100,
      }
      ```

7. Single Asset Order
    - Function: `SingleAssetDeposit` 
    ```
      {
        sender: walletAddress,
        poolId: <pool-id-here>,
        token: deposit,
        timeoutHeight: 100,
        timeoutTimestamp: 100,
      }
      ```

8. Withdraw Asset
    Withdraw asset has two steps:
    1. Increase allowance: Message will sent to lp token address of that pool.
    - Function: `IncreaseAllowance`   
    ```
        {
            spender: getWasmChainIDContractAddress(chain.chainID),
            amount: amount,
        }
    ```

    2. Call to ics101 contract address
    - Function: `MultiAssetWithdraw` 
    ```
        {
            poolId: poolItem.id,
            receiver: nativeAddress,
            counterpartyReceiver: counterPartyAddress,
            poolToken: {
                denom: poolItem.id,
                amount: amount,
            },
            timeoutHeight: 100,
            timeoutTimestamp: 100,
        }
      ```

9. Swap
    - Function: `MultiAssetWithdraw` 
    ```
      {
        sender: nativeAddress,
        swapType: 'LEFT',
        poolId: swapStore.selectedPool?.id,
        tokenIn, // Type is `Coin`
        tokenOut,// Type is `Coin`
        slippage: 1000,
        recipient: remoteAddress,
        timeoutHeight: 100,
        timeoutTimestamp: 100,
      }
      ```

For more information about how to call contract. Please refer to [Code](https://github.com/sideprotocol/sidex-ui-priviate/tree/dev/src/api/wasm/services)

## Query Interfaces