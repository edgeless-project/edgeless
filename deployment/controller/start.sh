#!/bin/bash

domain_register_port=(${DOMAIN_REGISTER_ENDPOINT//:/ })
cat > controller.toml << EOF
controller_url = "http://0.0.0.0:7001"
domain_register_url = "http://0.0.0.0:${domain_register_port[1]}"
EOF
if [ "$SHOWCONF" == "1" ] ; then
    cat controller.toml
fi
/usr/src/myapp/edgeless/target/release/edgeless_con_d