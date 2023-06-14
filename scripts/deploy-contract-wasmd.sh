CHAIN_ID1="source-chain"
HOME1="~/.wasmd1"
CHAIN_ID2="target-chain"
HOME2="~/.wasmd2"
KEY1="main1"
KEY2="main2"
wasmd tx wasm store ../target/wasm32-unknown-unknown/release/ics100_swap.wasm --from $KEY1 -y -b block --keyring-backend test --home $HOME1 --chain-id $CHAIN_ID1 --gas-prices 0.025stake --gas auto --gas-adjustment 1.3
wasmd tx wasm store ../target/wasm32-unknown-unknown/release/ics100_swap.wasm --from $KEY2 -y -b block --keyring-backend test --home $HOME2 --chain-id $CHAIN_ID2 --gas-prices 0.025stake --gas auto --gas-adjustment 1.3