CHAIN_ID2="target-chain"
TXFLAG2="--chain-id $CHAIN_ID2 --gas-prices 0.025stake --gas auto --gas-adjustment 1.3"
HOME2="~/.wasmd2"
KEY_TEST="--keyring-backend test"
MAIN2_KEY="track judge phrase loud step uncle play ridge more crawl dragon gospel enjoy ostrich mistake brush have glide arrive favorite vague food invest labor"
RES2_KEY="correct wage rebel kitten bunker sense shrimp pen library sphere expect before sport seminar sword vibrant antenna option poverty spring bench first addict invite"
SRES_KEY="interest horror shock refuse end frown master pool during antique desk inquiry impact random robot wet sword credit luxury brain hope proud entire local"

wasmd init my-node --chain-id $CHAIN_ID2 --home $HOME2
wasmd keys add main2 --keyring-backend test --home $HOME2 --recover
wasmd keys add res2 --keyring-backend test --home $HOME2 --recover
wasmd keys add validator2 --keyring-backend test --home $HOME2
wasmd add-genesis-account $(wasmd keys show main2 -a --keyring-backend test --home $HOME2) 10000000000stake,1000000000000token --home $HOME2 --keyring-backend test
wasmd add-genesis-account $(wasmd keys show validator2 -a --keyring-backend test --home $HOME2) 10000000000stake,1000000000000token --home $HOME2 --keyring-backend test
wasmd gentx validator2 1000000000stake --chain-id $CHAIN_ID2 --home $HOME2 --keyring-backend test
wasmd collect-gentxs --home $HOME2
wasmd validate-genesis --home $HOME2

sidechaind keys add sres --keyring-backend test --recover 
# wasmd start --home $HOME2