HOME1="/Users/ghostprince/.wasmd1"
HOME2="/Users/ghostprince/.wasmd2"
CONTRACT1=""
CONTRACT2="wasm14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s0phg4d"
LIST_QUERY='{"list": {}}'
# echo "List of atomic swaps on source chain"
# wasmd query wasm contract-state smart $CONTRACT1 "$LIST_QUERY" --output json --home $HOME1
echo "\nList of atomic swaps on target chain"
wasmd query wasm contract-state smart $CONTRACT2 "$LIST_QUERY" --output json --home $HOME2
echo "\nList of atomic swaps on sidechain"
sidechaind query ibc-swap orders