HOME1="/Users/ghostprince/.wasmd1"
HOME2="/Users/ghostprince/.wasmd2"
CONTRACT1="wasm1wn625s4jcmvk0szpl85rj5azkfc6suyvf75q6vrddscjdphtve8s5lsurx"
CONTRACT2="wasm1eyfccmjm6732k7wp4p6gdjwhxjwsvje44j0hfx8nkgrm8fs7vqfsuw7sel"
LIST_QUERY='{"list": {}}'
wasmd query wasm contract-state smart $CONTRACT1 "$LIST_QUERY" --output json --home $HOME1
wasmd query wasm contract-state smart $CONTRACT2 "$LIST_QUERY" --output json --home $HOME2