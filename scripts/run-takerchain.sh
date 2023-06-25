CHAIN_ID2="target-chain"
TXFLAG2="--chain-id $CHAIN_ID2 --gas-prices 0.025stake --gas auto --gas-adjustment 1.3"
HOME2="~/.wasmd2"
KEY_TEST="--keyring-backend test"
wasmd init my-node --chain-id $CHAIN_ID2 --home $HOME2
wasmd keys add main2 --keyring-backend test --home $HOME2
wasmd keys add validator2 --keyring-backend test --home $HOME2
wasmd add-genesis-account $(wasmd keys show main2 -a --keyring-backend test --home $HOME2) 10000000000stake,1000000000000token --home $HOME2 --keyring-backend test
wasmd add-genesis-account $(wasmd keys show validator2 -a --keyring-backend test --home $HOME2) 10000000000stake,1000000000000token --home $HOME2 --keyring-backend test
wasmd gentx validator2 1000000000stake --chain-id $CHAIN_ID2 --home $HOME2 --keyring-backend test
wasmd collect-gentxs --home $HOME2
wasmd validate-genesis --home $HOME2
# wasmd start --home $HOME2