CHAIN_ID1="source-chain"
HOME1="~/.wasmd1"
CHAIN_ID2="target-chain"
HOME2="~/.wasmd2"
KEY1="main1"
KEY2="main2"
CODE1="8"
CODE2="1"
LABEL="ics100 contract"
INIT='{}'
PATH_NAME="ics100_a"
SRC_PORT="swap"
VERSION="ics100-1"

# echo "\nCargo Test"
# cargo test -- --show-output

# echo "\nBuild contract"
# RUSTFLAGS="-C link-arg=-s" cargo build --release --target=wasm32-unknown-unknown --locked

# echo "\nDeploy to wasmd chain"
# # wasmd tx wasm store ../target/wasm32-unknown-unknown/release/ics100_swap.wasm --from $KEY1 -y -b block --keyring-backend test --home $HOME1 --chain-id $CHAIN_ID1 --gas-prices 0.025stake --gas auto --gas-adjustment 1.3
# wasmd tx wasm store ../target/wasm32-unknown-unknown/release/ics100_swap.wasm --from $KEY2 -y -b block --keyring-backend test --home $HOME2 --chain-id $CHAIN_ID2 --gas-prices 0.025stake --gas auto --gas-adjustment 1.3

# echo "\nList wasmd code"
# wasmd query wasm list-code --home $HOME2

# echo "\nInit wasm contract"
# wasmd tx wasm instantiate $CODE2 "$INIT" --from $KEY2 --chain-id $CHAIN_ID2 --label "$LABEL" --no-admin --keyring-backend test --home $HOME2

CONTRACT=$(wasmd query wasm list-contract-by-code $CODE2 --home $HOME2 --output json | jq -r '.contracts[-1]')

echo "\nContract address is $CONTRACT"

echo "\nCreate rly channel"
DST_PORT="wasm.$CONTRACT"
rly tx channel $PATH_NAME --src-port "$SRC_PORT" --dst-port "$DST_PORT" --version "$VERSION"