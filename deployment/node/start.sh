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
metrics_url = "http://127.0.0.1:7003"
orchestrator_url = "http://$ORCHESTRATOR_ENDPOINT"

EOF

if [ "$NODE_TYPE" == "WASM" ] ; then
    cat >> node.toml << EOF
    [wasm_runtime]
    enabled = true

    [user_node_capabilities]
    labels = $LABELS
EOF
elif [ $"$NODE_TYPE" == "FILE_LOG" ] ; then
    cat >> node.toml << EOF
[resources]
file_log_provider = "file-log-1"
EOF
else
    echo "invalid NODE_TYPE: $NODE_TYPE"
    exit 1
fi

if [ "$SHOWCONF" != "1" ] ; then
    cat node.toml
fi
/usr/src/myapp/edgeless/target/release/edgeless_node_d