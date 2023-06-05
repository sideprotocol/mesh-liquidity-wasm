HOME2="/Users/ghostprince/.wasmd2"
CONTRACT2="wasm14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s0phg4d"
KEY2="main2"
CHAIN_ID2="target-chain"

# Create make swap msg on source chain
wasmd tx wasm execute $CONTRACT2 '{"MakeSwap": {"source_port": "wasm.wasm14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s0phg4d", "source_channel": "channel-3",     "sell_token":     {"amount": "100", "denom": "token"},     "buy_token": { "amount": "100", "denom": "umlg" },     "maker_address": "wasm1ts2jqyjjee9dxxhljchx2kg2y248qs85pfvle6", "maker_receiving_address": "wasm1rwpfh06uye8zw6wm5yp93pw07va4c5v56kcjjd",  "desired_taker":"",    "create_timestamp": 1683279635,     "expiration_timestamp": 1693399749000000000,     "timeout_height": { "revision_number": 0, "revision_height": 99999999 },     "timeout_timestamp": 1693399799000000000  }}' --from $KEY2 --keyring-backend test --home "$HOME2" --chain-id $CHAIN_ID2 --gas-prices 0.025stake --gas auto --gas-adjustment 1.3 --amount 100token --trace