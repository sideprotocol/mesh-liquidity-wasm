PATH_NAME="ics100"
SRC_PORT="wasm.wasm1tqwwyth34550lg2437m05mjnjp8w7h5ka7m70jtzpxn4uh2ktsmqt0n86u"
DST_PORT="wasm.wasm10qt8wg0n7z740ssvf3urmvgtjhxpyp74hxqvqt7z226gykuus7eqfe7pg9"
VERSION="ics100-1"

# Create a new go relayer channel in the path
rly tx channel $PATH_NAME --src-port "$SRC_PORT" --dst-port "$DST_PORT" --version "$VERSION"