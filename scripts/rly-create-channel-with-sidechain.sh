PATH_NAME="ics100_b"
SRC_PORT="swap"
DST_PORT="wasm.wasm1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrqr5j2ht"
VERSION="ics100-1"

# Create a new go relayer channel in the path
rly tx channel $PATH_NAME --src-port "$SRC_PORT" --dst-port "$DST_PORT" --version "$VERSION"