HOME1="/Users/ghostprince/.wasmd1"
HOME2="/Users/ghostprince/.wasmd2"
CONTRACT1="wasm1tqwwyth34550lg2437m05mjnjp8w7h5ka7m70jtzpxn4uh2ktsmqt0n86u"
CONTRACT2="wasm1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrqr5j2ht"
KEY1="main1"
KEY2="main2"
CHAIN_ID1="source-chain"
CHAIN_ID2="target-chain"

# Create make swap msg on source chain
wasmd tx wasm execute $CONTRACT2 '{"MakeSwap": {"source_port": "wasm.wasm1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrqr5j2ht", "source_channel": "channel-3",     "sell_token":     {"amount": "100", "denom": "token"},     "buy_token": { "amount": "100", "denom": "aside" },     "maker_address": "wasm1kj2t5txvwznrdx32v6xsw46yqztsyahqwxwlve", "maker_receiving_address": "side17q04dvfl23m4cf9luhsfk4gng396d2cpwgzmny",  "desired_taker":"",    "create_timestamp": 1683279635,     "expiration_timestamp": 1693399749000000000,     "timeout_height": { "revision_number": 0, "revision_height": 99999999 },     "timeout_timestamp": 1693399799000000000  }}' --from $KEY2 --keyring-backend test --home "$HOME2" --chain-id $CHAIN_ID2 --gas-prices 0.025stake --gas auto --gas-adjustment 1.3 --amount 100token --trace