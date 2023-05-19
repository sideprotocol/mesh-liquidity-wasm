HOME1="/Users/ghostprince/.wasmd1"
HOME2="/Users/ghostprince/.wasmd2"
CHAIN_ID="sidechain_7070-1"
SWAPID="294139cd2bfd6952010184a376a5a675834c8469045609f5f5b192a5e4b8f380"
KEY1="main1"
KEY2="main2"
BOB="bob"

sidechaind tx ibc-swap take $SWAPID 100aside  wasm1fll0djfrcpkhszxzf4lfg6fp7eu6ywxqcn6pnx --from $BOB --keyring-backend test --chain-id $CHAIN_ID --gas-prices 0.01aside --gas auto --gas-adjustment 1.2 --trace --packet-timeout-height "0-9999994" --packet-timeout-timestamp "1693399799000000000"