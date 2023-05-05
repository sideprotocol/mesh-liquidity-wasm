HOME1="/Users/ghostprince/.wasmd1"
CONTRACT1="wasm1wn625s4jcmvk0szpl85rj5azkfc6suyvf75q6vrddscjdphtve8s5lsurx"
CONTRACT2="wasm1eyfccmjm6732k7wp4p6gdjwhxjwsvje44j0hfx8nkgrm8fs7vqfsuw7sel"
KEY1="main1"
CHAIN_ID1="source-chain"

# Create make swap msg on source chain
wasmd tx wasm execute $CONTRACT1 '{"MakeSwap": {"source_port": "wasm.wasm1wn625s4jcmvk0szpl85rj5azkfc6suyvf75q6vrddscjdphtve8s5lsurx", "source_channel": "channel-6",     "sell_token":     {      "native": [{        "amount": "100", "denom": "stake"      }]    },     "buy_token": {      "native": [{"amount": "100", "denom": "token"}]    },     "maker_address": "wasm15f0j8n2pmet97zaztucpsnxgz7gmrtruvh5ayt", "maker_receiving_address": "wasm15f0j8n2pmet97zaztucpsnxgz7gmrtruvh5ayt",      "creation_timestamp": "1683279635000000000",     "expiration_timestamp": "1683289635000000000",     "timeout_height": 200,     "timeout_timestamp": "1683289635000000000"  }}' --from $KEY1 --keyring-backend test --home "$HOME1" --chain-id $CHAIN_ID1 --gas-prices 0.025stake --gas auto --gas-adjustment 1.3 --amount 100stake --trace