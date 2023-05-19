INIT='{}'
CODE_ID="4848"
LABEL="ics100 contract"
KEY="wallet"

wasmd tx wasm instantiate $CODE_ID "$INIT" --from $KEY --chain-id malaga-420 --label "$LABEL" --no-admin --keyring-backend test --node https://rpc.malaga-420.cosmwasm.com:443 --gas-prices 0.25umlg --gas auto --gas-adjustment 1.3 -y

# wasmd query wasm list-contract-by-code $CODE2 --node https://rpc.malaga-420.cosmwasm.com:443 --chain-id malaga-420 --output json | jq -r '.contracts[-1]'