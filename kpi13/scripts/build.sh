#!/bin/bash

echo "Building edgeless and functions for local (non-docker) deployment."
EDGELESS_ROOT=../..

# rebuild edgeless if necessary
if [ ! -f ../build/edgeless_inabox ] || [[ "$1" == "--build" ]]; then
    pushd $EDGELESS_ROOT || exit
    cargo build --release
    popd || exit
    cp "$EDGELESS_ROOT"/target/release/edgeless_inabox ../build/
    cp "$EDGELESS_ROOT"/target/release/edgeless_node_d ../build/
    cp "$EDGELESS_ROOT"/target/release/edgeless_con_d ../build/
    cp "$EDGELESS_ROOT"/target/release/edgeless_orc_d ../build/
    cp "$EDGELESS_ROOT"/target/release/edgeless_cli ../build/
fi

# rebuild functions
pushd ../build || exit
./edgeless_cli function build ../functions/work_splitter/function.json
./edgeless_cli function build ../functions/calculator/function.json
popd || exit
