#!/bin/bash
BIN_DIR=target/release
TARGET=dda_evaluation/binaries

cd ../../edgeless || exit
cargo build --release
if [ $? -eq 0 ]; then
    echo edgeless build ok
else
    echo FAIL
    exit 1
fi

cd ../

# build edgeless
cp edgeless/$BIN_DIR/edgeless_benchmark ./$TARGET
cp edgeless/$BIN_DIR/edgeless_cli ./$TARGET
cp edgeless/$BIN_DIR/edgeless_inabox ./$TARGET

cd dda_evaluation || exit
# build functions needed for the dda_evaluation
./binaries/edgeless_cli function build ./functions/dda_chain_first/function.json
./binaries/edgeless_cli function build ./functions/dda_chain_mid/function.json
./binaries/edgeless_cli function build ./functions/dda_chain_last/function.json