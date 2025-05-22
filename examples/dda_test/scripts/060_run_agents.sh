#!/bin/bash
# SPDX-FileCopyrightText: © 2024 Siemens AG
# SPDX-License-Identifier: MIT
# Runs a set of agents which interact with edgeless workflows by sending events,
# actions, etc. Currently, 2 agents are started (with 1 DDA in edgeless, this
# gives us nice quorum properties of 3 nodes).
cd agent/
go build .
cd ..

# sleep for 5 after starting the agents to make sure that they have been
# successfully started
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
        kill -9 $pid #2>/dev/null
        echo "sent SIGINT to $pid"
    done

    # Wait for processes to exit and clean up
    for pid in "${pids[@]}"; do
        wait $pid #2>/dev/null
        echo "exited $pid"
    done
}

trap kill_all_processes SIGINT

for i in $(seq 1 5); do
    start_process ./agent/agent --id $i
    # we need to let the agents join gradually due to the nature of implemented
    # raft algortihm in dda 
    sleep 3
done

for pid in "${pids[@]}"; do
    wait $pid
done
