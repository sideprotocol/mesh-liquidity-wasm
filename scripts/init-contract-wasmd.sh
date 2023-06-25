INIT='{}'
CHAIN_ID1="source-chain"
HOME1="~/.wasmd1"
CHAIN_ID2="target-chain"
HOME2="~/.wasmd2"
KEY1="main1"
KEY2="main2"
CODE1="7"
CODE2="5"
LABEL="ics100 contract"

# Init ics100 contract on source chain
wasmd tx wasm instantiate $CODE1 "$INIT" --from $KEY1 --chain-id $CHAIN_ID1 --label "$LABEL" --no-admin --keyring-backend test --home $HOME1
# Init ics100 contract on target chain
wasmd tx wasm instantiate $CODE2 "$INIT" --from $KEY2 --chain-id $CHAIN_ID2 --label "$LABEL" --no-admin --keyring-backend test --home $HOME2