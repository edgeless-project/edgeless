#!/bin/bash
# SPDX-FileCopyrightText: © 2024 Siemens AG
# SPDX-License-Identifier: MIT

RUST_LOG=info ../../../target/debug/edgeless_orc_d

pids=()
start_process() {
    "$@" &
    pid=$!
    pids+=($pid)
    echo "started $pid ($@)"
}

kill_all_processes() {
    echo "exiting"
    for pid in "${pids[@]}"; do
        kill $pid 2>/dev/null
        echo "exited $pid"
    done
}

trap kill_all_processes SIGINT

start_process ./030_run_edgeless_ctrl.sh
start_process ./031_run_edgeless_orc.sh
start_process ./032_run_edgeless_node.sh

for pid in "${pids[@]}"; do
    wait $pid
done