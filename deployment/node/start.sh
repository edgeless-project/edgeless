#!/bin/bash

agent_addr_port=(${AGENT_ENDPOINT//:/ })
invocation_addr_port=(${INVOCATION_ENDPOINT//:/ })

cat > node.toml << EOF
[general]
node_id = "$(uuid)"
agent_url = "http://0.0.0.0:${agent_addr_port[1]}"
agent_url_announced = "http://$AGENT_ENDPOINT"
invocation_url = "http://0.0.0.0:${invocation_addr_port[1]}"
invocation_url_announced = "http://$INVOCATION_ENDPOINT"
orchestrator_url = "http://$ORCHESTRATOR_ENDPOINT"

[telemetry]
metrics_url = "http://0.0.0.0:7003"
log_level = "info"
performance_samples = true

EOF

if [ "$NODE_TYPE" == "WASM" ] ; then
    cat >> node.toml << EOF
[wasm_runtime]
enabled = true

[user_node_capabilities]
labels = $LABELS
num_cpus = 1
clock_freq_cpu = 1000
num_cores = $NUM_CORES
EOF

# node with the file log does not offer a wasm runtime
elif [ $"$NODE_TYPE" == "FILE_LOG" ] ; then
    cat >> node.toml << EOF
[resources]
file_log_provider = "file-log-1"
EOF

# DDA type activates the dda resource provider and wasm runtime
elif [ $"$NODE_TYPE" == "DDA" ] ; then
    cat >> node.toml << EOF
[wasm_runtime]
enabled = true

[user_node_capabilities]
labels = $LABELS
num_cpus = 1
clock_freq_cpu = 1000
num_cores = $NUM_CORES

[resources]
dda_provider = "dda-1"
dda_url = "http://127.0.0.1:12000"
EOF

else
    echo "invalid NODE_TYPE: $NODE_TYPE"
    exit 1
fi

if [ "$SHOWCONF" == "1" ] ; then
    cat node.toml
fi
/usr/src/myapp/edgeless/target/release/edgeless_node_d