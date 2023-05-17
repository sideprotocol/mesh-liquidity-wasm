HOME1="/Users/ghostprince/.wasmd1"
HOME2="/Users/ghostprince/.wasmd2"
CONTRACT1="wasm1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrqr5j2ht"
CONTRACT2="wasm1pvrwmjuusn9wh34j7y520g8gumuy9xtl3gvprlljfdpwju3x7ucsfg5rpz"
KEY="bob"
CHAIN_ID="sidechain_7070-1"

# Create make swap msg on sidechain
#sidechaind tx ibc-swap make '{"MakeSwap": {"source_port": "swap", "source_channel": "channel-3",     "sell_token":     {      "native": [{        "amount": "100", "denom": "aside"      }]    },     "buy_token": {      "native": [{"amount": "100", "denom": "stake"}]    },     "maker_address": "side1lqd386kze5355mgpncu5y52jcdhs85ckj7kdv0", "maker_receiving_address": "side1lqd386kze5355mgpncu5y52jcdhs85ckj7kdv0",      "creation_timestamp": "1683279635000000000",     "expiration_timestamp": "1693399749000000000",     "timeout_height": 200,     "timeout_timestamp": "1693399749000000000"  }}' --from $KEY --keyring-backend test --chain-id $CHAIN_ID --gas-prices 0.01aside --gas auto --gas-adjustment 1.2 --trace

sidechaind tx ibc-swap make channel-6 100aside side1lqd386kze5355mgpncu5y52jcdhs85ckj7kdv0 100stake --from $KEY --keyring-backend test --chain-id $CHAIN_ID --gas-prices 0.01aside --gas auto --gas-adjustment 1.2 --trace