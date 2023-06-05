HOME1="/Users/ghostprince/.wasmd1"
HOME2="/Users/ghostprince/.wasmd2"
CONTRACT1="wasm1tqwwyth34550lg2437m05mjnjp8w7h5ka7m70jtzpxn4uh2ktsmqt0n86u"
CONTRACT2="wasm1pvrwmjuusn9wh34j7y520g8gumuy9xtl3gvprlljfdpwju3x7ucsfg5rpz"
KEY1="main1"
KEY2="main2"
CHAIN_ID1="source-chain"
CHAIN_ID2="target-chain"

# Create make swap msg on source chain
wasmd tx wasm execute $CONTRACT1 '{"MakeSwap": {"source_port": "wasm.wasm1tqwwyth34550lg2437m05mjnjp8w7h5ka7m70jtzpxn4uh2ktsmqt0n86u", "source_channel": "channel-7",     "sell_token":     {      "native": [{        "amount": "100", "denom": "stake"      }]    },     "buy_token": {      "native": [{"amount": "100", "denom": "token"}]    },     "maker_address": "wasm15f0j8n2pmet97zaztucpsnxgz7gmrtruvh5ayt", "maker_receiving_address": "wasm15f0j8n2pmet97zaztucpsnxgz7gmrtruvh5ayt",      "creation_timestamp": "1683279635000000000",     "expiration_timestamp": "1693399735000000000",     "timeout_height": 200,     "timeout_timestamp": "1693399735000000000"  }}' --from $KEY1 --keyring-backend test --home "$HOME1" --chain-id $CHAIN_ID1 --gas-prices 0.025stake --gas auto --gas-adjustment 1.3 --amount 100stake --trace