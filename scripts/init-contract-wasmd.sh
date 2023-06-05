INIT='{}'
CHAIN_ID1="source-chain"
HOME1="~/.wasmd1"
CHAIN_ID2="target-chain"
HOME2="~/.wasmd2"
KEY1="main1"
KEY2="main2"
CODE1="8"
CODE2="2"
LABEL="ics100 contract"

# Init ics100 contract on source chain
# wasmd tx wasm instantiate $CODE1 "$INIT" --from $KEY1 --chain-id $CHAIN_ID1 --label "$LABEL" --no-admin --keyring-backend test --home $HOME1
# Init ics100 contract on target chain
wasmd tx wasm instantiate $CODE2 "$INIT" --from $KEY2 --chain-id $CHAIN_ID2 --label "$LABEL" --no-admin --keyring-backend test --home $HOME2

# wasmd query wasm list-contract-by-code $CODE2 --home $HOME2 --output json | jq -r '.contracts[-1]'