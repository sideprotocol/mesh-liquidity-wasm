HOME1="/Users/ghostprince/.wasmd1"
HOME2="/Users/ghostprince/.wasmd2"
CONTRACT2="wasm17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9jfksztgw5uh69wac2pgsm0v070"
CHAINID1="source-chain"
CHAINID2="target-chain"
SWAPID="42b3d7998f40c595cb14615ec68e5aa5f4083ae4d33f94b19adcb67281e88ea3"
KEY1="main1"
KEY2="main2"
wasmd tx wasm execute $CONTRACT2 '{"TakeSwap": { "order_id": "d1c14653d2a24acdbff4a336f56ef193d12ee100c21f7dd929e4969ccb619793", "sell_token": {"amount": "100", "denom": "stake"}, "taker_address": "wasm1ts2jqyjjee9dxxhljchx2kg2y248qs85pfvle6", "taker_receiving_address": "side19kl420hmy0m9d0uul67kn20xnnkgkxmwg49rh9", "create_timestamp": 1683279635,  "timeout_height": { "revision_number": 0, "revision_height": 99999991 },     "timeout_timestamp": 1693399799000000000 }}' --from $KEY2 --keyring-backend test --home $HOME2 --chain-id $CHAINID2 --gas-prices 0.025stake --gas auto --gas-adjustment 1.3 --amount 100stake --trace