wasmd keys add wallet --keyring-backend test
wasmd keys add wallet2 --keyring-backend test
source <(curl -sSL https://raw.githubusercontent.com/CosmWasm/testnets/master/malaga-420/defaults.env)
JSON=$(jq -n --arg addr $(wasmd keys show -a wallet --keyring-backend test) '{"denom":"umlg","address":$addr}') && curl -X POST --header "Content-Type: application/json" --data "$JSON" https://faucet.malaga-420.cosmwasm.com/credit
JSON=$(jq -n --arg addr $(wasmd keys show -a wallet2 --keyring-backend test) '{"denom":"umlg","address":$addr}') && curl -X POST --header "Content-Type: application/json" --data "$JSON" https://faucet.malaga-420.cosmwasm.com/credit
NODE="--node $RPC"
TXFLAG="${NODE} --chain-id ${CHAIN_ID} --gas-prices 0.25${FEE_DENOM} --gas auto --gas-adjustment 1.3"
wasmd query bank total --node https://rpc.malaga-420.cosmwasm.com:443
wasmd query bank balances $(wasmd keys show -a wallet --keyring-backend test) --node https://rpc.malaga-420.cosmwasm.com:443
wasmd query bank balances $(wasmd keys show -a wallet2 --keyring-backend test) --node https://rpc.malaga-420.cosmwasm.com:443
wasmd tx wasm store target/wasm32-unknown-unknown/release/ics100_swap.wasm --from wallet -y -b block --keyring-backend test --chain-id malaga-420 --node https://rpc.malaga-420.cosmwasm.com:443 --gas-prices 0.25umlg --gas auto --gas-adjustment 1.3