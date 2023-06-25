HOME1="~/.wasmd1"
HOME2="~/.wasmd2"
CONTRACT1="wasm1tqwwyth34550lg2437m05mjnjp8w7h5ka7m70jtzpxn4uh2ktsmqt0n86u"
CONTRACT2="wasm1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrqr5j2ht"
SWAPID="2708f44389952a0e75cf3e77cb926772668ba829866a00d4c5297a55466e46e7"
LIST_QUERY='{"list": {}}'
# echo "List of atomic swaps on source chain"
# wasmd query wasm contract-state smart $CONTRACT1 "$LIST_QUERY" --output json --home $HOME1
echo "\nList of atomic swaps on target chain"
wasmd query wasm contract-state smart $CONTRACT2 "$LIST_QUERY" --output json --home $HOME2
echo "\nList of atomic swaps on sidechain"
sidechaind query ibc-swap orders