HOME1="/Users/ghostprince/.wasmd1"
CONTRACT1="wasm1466nf3zuxpya8q9emxukd7vftaf6h4psr0a07srl5zw74zh84yjqeam05w"
CONTRACT2="wasm1zwv6feuzhy6a9wekh96cd57lsarmqlwxdypdsplw6zhfncqw6ftqm3x37s"
KEY1="main1"
CHAIN_ID1="source-chain"

# Create make swap msg on source chain
wasmd tx wasm execute $CONTRACT1 '{"CancelSwap": {"order_id": "66a661747fede287cc6f6439eb3597683ae973a9c1b738bd9430798ae71ff013", "maker_address": "wasm15f0j8n2pmet97zaztucpsnxgz7gmrtruvh5ayt",  "creation_timestamp": "1683125169",   "timeout_height": 200,     "timeout_timestamp": 1683126169  }}' --from $KEY1 --keyring-backend test --home "$HOME1" --chain-id $CHAIN_ID1 --gas-prices 0.025stake --gas auto --gas-adjustment 1.3 --trace