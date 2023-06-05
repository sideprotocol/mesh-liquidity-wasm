HOME1="/Users/ghostprince/.wasmd1"
HOME2="/Users/ghostprince/.wasmd2"
CHAIN_ID="sidechain_7070-1"
SWAPID="9b66a7a3223317a81435c4ef5e31f2120205ddaebf53ff5f4617b2db89d863c4"
KEY1="main1"
KEY2="main2"
BOB="bob"

sidechaind tx ibc-swap take $SWAPID 100aside  wasm1fll0djfrcpkhszxzf4lfg6fp7eu6ywxqcn6pnx --from $BOB --keyring-backend test --chain-id $CHAIN_ID --gas-prices 0.01aside --gas auto --gas-adjustment 1.2 --trace --packet-timeout-height "0-9999994" --packet-timeout-timestamp "1693399799000000000"