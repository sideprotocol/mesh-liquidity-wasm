HOME1="~/.wasmd1"
HOME2="~/.wasmd2"
CONTRACT1="wasm1wn625s4jcmvk0szpl85rj5azkfc6suyvf75q6vrddscjdphtve8s5lsurx"
CONTRACT2="wasm1eyfccmjm6732k7wp4p6gdjwhxjwsvje44j0hfx8nkgrm8fs7vqfsuw7sel"
CHAINID1="source-chain"
CHAINID2="target-chain"
SWAPID="825e9cfb35eac2d3e6b8add9dc0134308e7c3fbc067b46e43c86503996a48c60"
KEY2="main2"
wasmd tx wasm execute $CONTRACT2 '{"TakeSwap": { "order_id": "825e9cfb35eac2d3e6b8add9dc0134308e7c3fbc067b46e43c86503996a48c60", "sell_token": { "native": [{"amount": "100", "denom": "token"}] }, "taker_address": "wasm19cmekqgu779n6hjpga7jyvl4gvrd8uhhksa27d", "taker_receiving_address": "wasm19cmekqgu779n6hjpga7jyvl4gvrd8uhhksa27d", "creation_timestamp": "1683279635000000000", "timeout_height": 200, "timeout_timestamp": "1683289635000000000" }}' --from $KEY2 --keyring-backend test --home $HOME2 --chain-id $CHAINID2 --gas-prices 0.025stake --gas auto --gas-adjustment 1.3 --amount 100token --trace