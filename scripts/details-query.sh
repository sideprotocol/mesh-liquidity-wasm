HOME1="~/.wasmd1"
HOME2="~/.wasmd2"
CONTRACT1="wasm1466nf3zuxpya8q9emxukd7vftaf6h4psr0a07srl5zw74zh84yjqeam05w"
CONTRACT2="wasm1zwv6feuzhy6a9wekh96cd57lsarmqlwxdypdsplw6zhfncqw6ftqm3x37s"
LIST_QUERY='{"details": {"id":"66a661747fede287cc6f6439eb3597683ae973a9c1b738bd9430798ae71ff013"}}'
echo "Query details in source chain"
wasmd query wasm contract-state smart $CONTRACT1 "$LIST_QUERY" --output json --home $HOME1
echo "\nQuery details in target chain"
wasmd query wasm contract-state smart $CONTRACT2 "$LIST_QUERY" --output json --home $HOME2