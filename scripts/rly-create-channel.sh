PATH_NAME="ics100"
SRC_PORT="wasm.wasm1wn625s4jcmvk0szpl85rj5azkfc6suyvf75q6vrddscjdphtve8s5lsurx"
DST_PORT="wasm.wasm1eyfccmjm6732k7wp4p6gdjwhxjwsvje44j0hfx8nkgrm8fs7vqfsuw7sel"
VERSION="ics100-1"

# Create a new go relayer channel in the path
rly tx channel $PATH_NAME --src-port "$SRC_PORT" --dst-port "$DST_PORT" --version "$VERSION"