#!/bin/bash

echo "Building edgeless and functions for local (non-docker) deployment."
CWD=$(pwd)
EDGELESS_ROOT=../../edgeless

# rebuild edgeless if necessary
if [ ! -f ../deployment/cfg-local-inabox/edgeless_inabox ] || [[ "$1" == "--build" ]]; then
    cd $EDGELESS_ROOT
    EDGELESS_ROOT=$(pwd)
    cargo build --release
    cd $CWD
    cp $EDGELESS_ROOT/target/release/edgeless_inabox ../build/
    cp $EDGELESS_ROOT/target/release/edgeless_node_d ../build/
    cp $EDGELESS_ROOT/target/release/edgeless_con_d ../build/
    cp $EDGELESS_ROOT/target/release/edgeless_orc_d ../build/
    cp $EDGELESS_ROOT/target/release/edgeless_cli ../build/
fi

# rebuild functions
cd ../build
./edgeless_cli function build ../functions/work_splitter/function.json
./edgeless_cli function build ../functions/calculator/function.json
