#!/bin/bash

# =========================================================== 
# Colors for logging
CLR_RED='\033[0;31m'
CLR_GREEN='\033[0;32m'
CLR_YELLOW='\033[0;33m'
CLR_RST='\033[0m'

# DEBUG prints
function echo_r(){ echo -e "${CLR_RED}$*${CLR_RST}"; }
function echo_g(){ echo -e "${CLR_GREEN}$*${CLR_RST}"; }
function echo_y(){ echo -e "${CLR_YELLOW}$*${CLR_RST}"; }
# =========================================================== 

logs="build.log build_functions.log edgeless_bal.log edgeless_con.log edgeless_orc.log edgeless_node.log my-local-file.log reading-errors.log"
confs="balancer.toml controller.toml orchestrator.toml node.toml cli.toml"
specialized_workflows="container dda_demo dda_test esp32_resources redis vector_mul matrix_mul ollama kafka_egress"

echo_y "> Checking for existing files"
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
    echo "The following files exist and will be overwritten if you continue: $existing_files"
    read -s -n 1 -p "Hit Ctrl+C to abort or press any key to continue"
    echo ""
    rm $logs $confs 2> /dev/null
fi

echo
echo_y "> Compiling"
cargo build >& build.log
if [ $? -ne 0 ] ; then
    echo_r "[error] compiling, bailing out"
    exit 1
fi

echo
echo_y "> Building the functions, if needed"
scripts/functions_build.sh
if [ $? -ne 0 ] ; then
    echo_r "[error] building some functions, bailing out"
    exit 1
fi

echo
echo_y "> Creating configuration files"
target/debug/edgeless_bal_d -t balancer.toml
target/debug/edgeless_con_d -t controller.toml
target/debug/edgeless_orc_d -t orchestrator.toml
target/debug/edgeless_node_d -t node.toml
target/debug/edgeless_cli -t cli.toml

echo
echo_y "> Starting orchestrator, controller, and a node"
pids=()
RUST_LOG=info target/debug/edgeless_bal_d >& edgeless_bal.log &
pids+=($!)
RUST_LOG=info target/debug/edgeless_con_d >& edgeless_con.log &
pids+=($!)
RUST_LOG=info target/debug/edgeless_orc_d >& edgeless_orc.log &
pids+=($!)
RUST_LOG=info target/debug/edgeless_node_d >& edgeless_node.log &
pids+=($!)

echo -n "waiting for the cluster to be ready"
while (true) ; do
    echo -n "."
    out=$(target/debug/edgeless_cli domain inspect domain-1 | grep "1 node")
    if [ "$out" != "" ] ; then
        break
    fi
    sleep 0.1
done
echo "done"

echo "starting workflows"
for workflow in $(find examples -type f -name "workflow*.json") ; do
    name=$(basename $(dirname $workflow))
    if [ "$RUN_SPECIALIZED_WORKFLOWS" != "1" ] ; then
        specialized=0
        for specialized_workflow in $specialized_workflows ; do
            if [[ "$workflow" == *"$specialized_workflow"* ]] ; then
                specialized=1
                break
            fi
        done
        if [ $specialized -eq 1 ] ; then
            echo "skipping specialized workflow '$workflow', use RUN_SPECIALIZED_WORKFLOWS=1 to run everything"
            continue
        fi
    fi

    echo -n "starting workflow $name: "
    uid=$(RUST_LOG=error target/debug/edgeless_cli workflow start $workflow | grep '-')
    if [ $? -eq 0 ] ; then
        echo_g "[started with ID $uid]"
    else
        echo_r "[error]"
    fi
    sleep 2
    echo -n "stopping workflow $name: "
    target/debug/edgeless_cli workflow stop $uid >& /dev/null
    if [ $? -eq 0 ] ; then
        echo_g "[done]"
    else
        echo_r "[error]"
    fi
done

echo
echo_y "> Cleaning up"
for pid in "${pids[@]}" ; do
    echo "killing $pid"
    kill $pid
done
wait

read -s -n 1 -p "press any key to remove all conf&log files (Ctrl+C if you want to keep them)"
echo ""
rm $logs $confs 2> /dev/null