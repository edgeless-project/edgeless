#!/bin/bash

orc_addr_port=(${ORCHESTRATOR_ENDPOINT//:/ })
agent_addr_port=(${AGENT_ENDPOINT//:/ })
invocation_addr_port=(${INVOCATION_ENDPOINT//:/ })

cat > orchestrator.toml << EOF
[general]
domain_id = "domain-1"
orchestrator_url = "http://0.0.0.0:${orc_addr_port[1]}"
orchestrator_url_announced = "http://$ORCHESTRATOR_ENDPOINT"
agent_url = "http://0.0.0.0:${agent_addr_port[1]}"
agent_url_announced = "http://$AGENT_ENDPOINT"
invocation_url = "http://0.0.0.0:${invocation_addr_port[1]}"
invocation_url_announced = "http://$INVOCATION_ENDPOINT"

[baseline]
orchestration_strategy = "Random"
keep_alive_interval_secs = 2

[proxy]
proxy_type = "Redis"
redis_url = "redis://$REDIS_ENDPOINT"

[collector]
collector_type = "Redis"
redis_url = "redis://$REDIS_ENDPOINT"
EOF

if [ "$SHOWCONF" == "1" ] ; then
    cat orchestrator.toml
fi
/usr/src/myapp/edgeless/target/release/edgeless_orc_d