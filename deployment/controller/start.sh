#!/bin/bash

cat > controller.toml << EOF
controller_url = "http://0.0.0.0:7001"
orchestrators = [
    { domain_id = "domain-1", orchestrator_url="http://$ORCHESTRATOR_ENDPOINT" }
]
EOF
if [ "$SHOWCONF" != "" ] ; then
    cat controller.toml
fi
/usr/src/myapp/edgeless/target/release/edgeless_con_d