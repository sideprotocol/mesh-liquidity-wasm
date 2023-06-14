CHAIN_ID1="source-chain"
TXFLAG1="--chain-id $CHAIN_ID1 --gas-prices 0.025stake --gas auto --gas-adjustment 1.3"
HOME1="~/.wasmd1"
KEY_TEST="--keyring-backend test"
wasmd init my-node --chain-id $CHAIN_ID1 --home $HOME1
wasmd keys add main1 --keyring-backend test --home $HOME1
wasmd keys add validator1 --keyring-backend test --home $HOME1
wasmd add-genesis-account $(wasmd keys show main1 -a --keyring-backend test --home $HOME1) 10000000000stake --home $HOME1 --keyring-backend test
wasmd add-genesis-account $(wasmd keys show validator1 -a --keyring-backend test --home $HOME1) 10000000000stake --home $HOME1 --keyring-backend test
wasmd gentx validator1 1000000000stake --chain-id $CHAIN_ID1 --home $HOME1 --keyring-backend test
wasmd collect-gentxs --home $HOME1
wasmd validate-genesis --home $HOME1
wasmd start --home $HOME1