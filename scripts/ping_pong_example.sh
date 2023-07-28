#!/bin/bash

logs="build.log edgeless_bal.log edgeless_con.log edgeless_orc.log edgeless_node.log"
confs="balancer.toml controller.toml orchestrator.toml node.toml cli.toml"

echo "checking for existing files"
existing_files=""
for log in $logs ; do
    if [ -r $log ] ; then
        existing_files="$existing_files $log"
    fi
done
for conf in $confs ; do
    if [ -r $conf ] ; then
        existing_files="$existing_files $conf"
    fi
done
if [ "$existing_files" != "" ] ; then
    echo "the following files exist and will be overwritten if you continue: $existing_files"
    read -s -n 1 -p "hit Ctrl+C to abort or press any key to continue"
    echo ""
    rm $logs $confs 2> /dev/null
fi

echo "compiling"
cargo build >& build.log
if [ $? -ne 0 ] ; then
    echo "error compiling, bailing out"
    exit 1
fi

echo "creating configuration files"
target/debug/edgeless_bal_d -t balancer.toml
target/debug/edgeless_con_d -t controller.toml
target/debug/edgeless_orc_d -t orchestrator.toml
target/debug/edgeless_node_d -t node.toml
target/debug/edgeless_cli -t cli.toml

echo "starting orchestrator, controller, and a node"
pids=()
RUST_LOG=info target/debug/edgeless_bal_d >& edgeless_bal.log & 
pids+=($!)
RUST_LOG=info target/debug/edgeless_con_d >& edgeless_con.log & 
pids+=($!)
RUST_LOG=info target/debug/edgeless_orc_d >& edgeless_orc.log & 
pids+=($!)
RUST_LOG=info target/debug/edgeless_node_d >& edgeless_node.log &
pids+=($!)

sleep 0.5

echo "building the functions, if needed"
if [ ! -r examples/ping_pong/ping/pinger.wasm ] ; then
    target/debug/edgeless_cli function build examples/ping_pong/ping/function.json >& pinger.log
fi
if [ ! -r examples/ping_pong/pong/ponger.wasm ] ; then
    target/debug/edgeless_cli function build examples/ping_pong/pong/function.json >& ponger.log
fi

if [[ ! -r examples/ping_pong/ping/pinger.wasm || ! -r examples/ping_pong/pong/ponger.wasm ]] ; then
    echo "could not build the functions, check logs"
else
    uid=$(target/debug/edgeless_cli workflow start examples/ping_pong/workflow.json)
    echo "workflow UID = $uid"
    echo "sleeping for 2 seconds"
    sleep 2
    echo "terminating the workflow"
    target/debug/edgeless_cli workflow stop $uid
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

read -s -n 1 -p "press any key to remove all conf&log files (Ctrl+C if you want to keep them)"
echo ""
rm $logs $confs 2> /dev/null