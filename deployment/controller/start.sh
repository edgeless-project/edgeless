#!/bin/bash

rm -f controller.toml
/usr/src/myapp/edgeless/target/release/edgeless_con_d -t controller.toml
sed -i \
    -e "s/127.0.0.1:7001/0.0.0.0:7001/" \
    -e "s/127.0.0.1:7011/$ORCHESTRATOR_ENDPOINT/"\
    controller.toml
/usr/src/myapp/edgeless/target/release/edgeless_con_d