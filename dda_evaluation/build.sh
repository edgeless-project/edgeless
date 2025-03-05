#!/bin/bash
cd ext
go build -C sensor
go build -C tester
go build -C actor
cd ..

# build the actor
docker build -t actor --platform linux/arm64 -f ./ext/actor/Dockerfile ext/actor

# build edgeless for docker containers
cd ..
cargo build --target=aarch64-unknown-linux-gnu --release
cp target/aarch64-unknown-linux-gnu/release/edgeless_node_d dda_evaluation/binaries
cp target/aarch64-unknown-linux-gnu/release/edgeless_con_d dda_evaluation/binaries
cp target/aarch64-unknown-linux-gnu/release/edgeless_orc_d dda_evaluation/binaries

# dockerfiles must be in root of this dir, due to docker context
cd dda_evaluation

docker build -t dda_node --platform linux/arm64 --build-arg BIN=./binaries -f Dockerfile.node .
docker build -t dda_orc --platform linux/arm64 --build-arg BIN=./binaries -f Dockerfile.orc .
docker build -t dda_con --platform linux/arm64 --build-arg BIN=./binaries -f Dockerfile.con .

# rebuild the functions
cd ..
# if we modify any of the imported libraries, we need to also remove the .wasm file
rm dda_evaluation/functions/dda_call/function.wasm
RUST_LOG=INFO target/debug/edgeless_cli function build dda_evaluation/functions/dda_call/function.json