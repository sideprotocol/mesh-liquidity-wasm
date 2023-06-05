OLD_PATH="ics100_d"
PATH_NAME="ics100_e"
SRC_PORT="swap"
DST_PORT="wasm.wasm17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9jfksztgw5uh69wac2pgsm0v070"
VERSION="ics100-1"
SRC_CHAIN_ID="sidechain_7070-1"
DST_CHAIN_ID="target-chain"

# Create a new go relayer channel in the path
rly paths delete $OLD_PATH
rly paths new $SRC_CHAIN_ID $DST_CHAIN_ID $PATH_NAME
rly paths list
rly tx link $PATH_NAME
rly tx channel $PATH_NAME --src-port "$SRC_PORT" --dst-port "$DST_PORT" --version "$VERSION"