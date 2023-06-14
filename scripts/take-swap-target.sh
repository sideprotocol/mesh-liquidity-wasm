HOME1="~/.wasmd1"
HOME2="~/.wasmd2"
CONTRACT2="wasm1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrqr5j2ht"
CHAINID1="source-chain"
CHAINID2="target-chain"
SWAPID="42b3d7998f40c595cb14615ec68e5aa5f4083ae4d33f94b19adcb67281e88ea3"
KEY1="main1"
KEY2="main2"
wasmd tx wasm execute $CONTRACT2 '{"TakeSwap": { "order_id": "42b3d7998f40c595cb14615ec68e5aa5f4083ae4d33f94b19adcb67281e88ea3", "sell_token": {"amount": "100", "denom": "stake"}, "taker_address": "wasm1ts2jqyjjee9dxxhljchx2kg2y248qs85pfvle6", "taker_receiving_address": "side19kl420hmy0m9d0uul67kn20xnnkgkxmwg49rh9", "create_timestamp": 1683279635,  "timeout_height": { "revision_number": 0, "revision_height": 99999999 },     "timeout_timestamp": 1693399799000000000 }}' --from $KEY2 --keyring-backend test --home $HOME2 --chain-id $CHAINID2 --gas-prices 0.025stake --gas auto --gas-adjustment 1.3 --amount 100stake --trace