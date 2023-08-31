#!/bin/bash

echo "creating a directory to store conf&log files for this example"
if [ -d ping_pong_example ] ; then
    echo "the following directory exists and it will be cleared if you continue: ping_pong_example"
    read -s -n 1 -p "hit Ctrl+C to abort or press any key to continue"
    rm -rf ping_pong_example 2> /dev/null
fi
mkdir ping_pong_example 2> /dev/null

echo "compiling"
cargo build >& ping_pong_example/build.log
if [ $? -ne 0 ] ; then
    echo "error compiling, bailing out"
    exit 1
fi

cd ping_pong_example

echo "creating configuration files"
../target/debug/edgeless_bal_d -t balancer.toml
../target/debug/edgeless_con_d -t controller.toml
../target/debug/edgeless_orc_d -t orchestrator.toml
../target/debug/edgeless_node_d -t node.toml
../target/debug/edgeless_cli -t cli.toml

echo "starting orchestrator, controller, and a node"
pids=()
RUST_LOG=info ../target/debug/edgeless_bal_d >& edgeless_bal.log & 
pids+=($!)
RUST_LOG=info ../target/debug/edgeless_con_d >& edgeless_con.log & 
pids+=($!)
RUST_LOG=info ../target/debug/edgeless_orc_d >& edgeless_orc.log & 
pids+=($!)
RUST_LOG=info ../target/debug/edgeless_node_d >& edgeless_node.log &
pids+=($!)

sleep 0.5

echo "building the functions, if needed"
if [ ! -r ../examples/ping_pong/ping/pinger.wasm ] ; then
    ../target/debug/edgeless_cli function build ../examples/ping_pong/ping/function.json >& pinger.log
fi
if [ ! -r ../examples/ping_pong/pong/ponger.wasm ] ; then
    ../target/debug/edgeless_cli function build ../examples/ping_pong/pong/function.json >& ponger.log
fi

if [[ ! -r ../examples/ping_pong/ping/pinger.wasm || ! -r ../examples/ping_pong/pong/ponger.wasm ]] ; then
    echo "could not build the functions, check logs"
else
    uid=$(../target/debug/edgeless_cli workflow start ../examples/ping_pong/workflow.json)
    echo "workflow UID = $uid"
    echo "sleeping for 2 seconds"
    sleep 2
    echo "terminating the workflow"
    ../target/debug/edgeless_cli workflow stop $uid
    if [ $? -ne 0 ] ; then
        echo "something when wrong when terminating the workflow"
    fi
    if [ "$(grep 'Got Reply' edgeless_node.log)" == "" ] ; then
        echo "the workflow execution failed"
    fi
fi

echo "cleaning up"
for pid in "${pids[@]}" ; do
    echo "killing $pid"
    kill $pid
done
wait

cd ..

read -s -n 1 -p "press any key to remove all conf&log files (Ctrl+C if you want to keep them)"
echo ""
rm -r ping_pong_example 2> /dev/null