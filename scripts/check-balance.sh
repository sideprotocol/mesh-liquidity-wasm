HOME1="~/.wasmd1"
HOME2="~/.wasmd2"
CHAINID1="source-chain"
CHAINID2="target-chain"
KEY1="main1"
KEY2="main2"
ADDRESS1="wasm15f0j8n2pmet97zaztucpsnxgz7gmrtruvh5ayt"
ADDRESS2="wasm19cmekqgu779n6hjpga7jyvl4gvrd8uhhksa27d"
CONTRACT1="wasm1466nf3zuxpya8q9emxukd7vftaf6h4psr0a07srl5zw74zh84yjqeam05w"
CONTRACT2="wasm1zwv6feuzhy6a9wekh96cd57lsarmqlwxdypdsplw6zhfncqw6ftqm3x37s"

# show balances
echo "Balance of main1 on source chain"
wasmd q bank balances $ADDRESS1 --home $HOME1 --chain-id $CHAINID1
echo "\nBalance of main1 on target chain"
wasmd q bank balances $ADDRESS1 --home $HOME2 --chain-id $CHAINID2

echo "\n\nBalance of main2 on source chain"
wasmd q bank balances $ADDRESS2 --home $HOME1 --chain-id $CHAINID1
echo "\nBalance of main2 on target chain"
wasmd q bank balances $ADDRESS2 --home $HOME2 --chain-id $CHAINID2

echo "\n\nBalance of contract1 on source chain"
wasmd q bank balances $CONTRACT1 --home $HOME1 --chain-id $CHAINID1
echo "\nBalance of contract2 on target chain"
wasmd q bank balances $CONTRACT2 --home $HOME2 --chain-id $CHAINID2
