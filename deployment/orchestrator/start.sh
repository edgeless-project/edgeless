#!/bin/bash

ports="7011 7121 7102"

rm -f controller.toml
/usr/src/myapp/edgeless/target/release/edgeless_orc_d -t orchestrator.toml
for port in $ports ; do
    sed -i -e "s/127.0.0.1:$port/0.0.0.0:$port/" orchestrator.toml
done
sed -i \
    -e 's/proxy_type = "None"/proxy_type = "Redis"/' \
    -e 's/collector_type = "None"/collector_type = "Redis"/' \
    -e "s/redis_url = \"\"/redis_url = \"redis:\/\/$REDIS_ENDPOINT\"/" \
    orchestrator.toml
if [ "$SHOWCONF" != "" ] ; then
    cat orchestrator.toml
fi
/usr/src/myapp/edgeless/target/release/edgeless_orc_d