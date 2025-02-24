#!/bin/bash

orc_addr_port=(${ORCHESTRATOR_ENDPOINT//:/ })
node_register_port=(${NODE_REGISTER_ENDPOINT//:/ })

# NOTE: kept in the same order as the struct defines them
cat > orchestrator.toml << EOF
[general]
domain_register_url = "http://$DOMAIN_REGISTER_ENDPOINT"
subscription_refresh_interval_sec = 2
domain_id = "domain-1"
orchestrator_url = "http://0.0.0.0:${orc_addr_port[1]}"
orchestrator_url_announced = "http://$ORCHESTRATOR_ENDPOINT"
node_register_url = "http://0.0.0.0:${node_register_port[1]}"

[baseline]
orchestration_strategy = "Random"

[proxy]
proxy_type = "Redis"
redis_url = "redis://$REDIS_ENDPOINT"

EOF

if [ "$SHOWCONF" == "1" ] ; then
    cat orchestrator.toml
fi
/usr/src/myapp/edgeless/target/release/edgeless_orc_d