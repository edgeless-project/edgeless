#!/bin/bash

cat > node.toml << EOF
[general]
node_id = "$(uuid)"
agent_url = "http://0.0.0.0:$AGENT_PORT"
agent_url_announced = "http://$AGENT_ENDPOINT"
invocation_url = "http://0.0.0.0:$INVOCATION_PORT"
invocation_url_announced = "http://$INVOCATION_ENDPOINT"
metrics_url = "http://127.0.0.1:7003"
orchestrator_url = "http://$ORCHESTRATOR_ENDPOINT"

[wasm_runtime]
enabled = true

[user_node_capabilities]
labels = $LABELS
EOF

if [ "$SHOWCONF" != "" ] ; then
    cat node.toml
fi
/usr/src/myapp/edgeless/target/release/edgeless_node_d