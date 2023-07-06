PATH_NAME="ics100"
SRC_PORT="wasm.wasm1wn625s4jcmvk0szpl85rj5azkfc6suyvf75q6vrddscjdphtve8s5lsurx"
DST_PORT="wasm.wasm1eyfccmjm6732k7wp4p6gdjwhxjwsvje44j0hfx8nkgrm8fs7vqfsuw7sel"
VERSION="ics100-1"

# Create a new go relayer channel in the path
rly tx channel $PATH_NAME --src-port "wasm.osmo1usde2wnww8qp5f4gjquyw2nukgz70y3elttfqsvxvs9ur889yn7s8nt68s" --dst-port "wasm.juno1r65gaut7qamn36pnq3c290grcdrrlm3cahfc82lt0kg3lk0qel0qauz74g" --version "ics101-1"

rly tx channel wasm-path --src-port "$SRC_PORT" --dst-port "$DST_PORT" --version "$VERSION"
