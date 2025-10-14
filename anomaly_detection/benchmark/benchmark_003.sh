#!/bin/bash

thishost="10.95.82.178"
nodes="10.95.82.180 10.95.82.179 10.95.82.190 10.95.82.191 10.95.82.192"
edgeless_root=$HOME/edgeless/

seeds="1 2 3 4 5 6 7 8 9 10"
orchestrations="Random RoundRobin"
interarrivals="60 30 20 10"

for seed in $seeds ; do
for orchestration in $orchestrations ; do
for interarrival in $interarrivals ; do

echo "Stop EDGELESS cluster if its still running"
for node in ${nodes} ; do
    ssh ${node} "killall edgeless_node_d"
done

killall edgeless_con_d
killall edgeless_orc_d
killall edgeless_bal_d
killall edgeless_node_d


additional_fields=$seed,$orchestration,$interarrival
additional_header=seed,orchestration,interarrival


sed -i "s/^orchestration_strategy\s*=\s*.*/orchestration_strategy = \"${orchestration}\"/" configs/orchestrator.toml
sed -i "s/^additional_fields\s*=\s*.*/additional_fields = \"${additional_fields}\"/" configs/orchestrator.toml
sed -i "s/^additional_header\s*=\s*.*/additional_header = \"${additional_header}\"/" configs/orchestrator.toml


echo "Start EDGELESS cluster"

nohup env RUST_LOG=info ${edgeless_root}/target/debug/edgeless_con_d -c configs/controller.toml   > logs/con-seed${seed}-${orchestration}-${interarrival}.log 2>&1 &
nohup env RUST_LOG=info ${edgeless_root}/target/debug/edgeless_orc_d -c configs/orchestrator.toml > logs/orc-seed${seed}-${orchestration}-${interarrival}.log 2>&1 &
nohup env RUST_LOG=info ${edgeless_root}/target/debug/edgeless_bal_d -c configs/balancer.toml     > logs/bal-seed${seed}-${orchestration}-${interarrival}.log 2>&1 &

for node in ${nodes}


do
    ssh ${node} "nohup env RUST_LOG=info edgeless/target/debug/edgeless_node_d -c configs/node.toml > logs/node-seed${seed}-${orchestration}-${interarrival}.log 2>&1 &"
done

echo "Starting the experiment"
RUST_LOG=info ${edgeless_root}/target/debug/edgeless_benchmark \
    --controller-url http://${thishost}:7001 \
    --orchestrator-url http://${thishost}:7011 \
    --bind-address ${thishost} \
    --arrival-model poisson \
    --warmup 0 \
    --duration 3600 \
    --lifetime 300 \
    --interarrival ${interarrival} \
    --seed ${seed} \
    --wf-type "map-reduce;100;1900;0;0;1;4;1;4;5000;50000;0;0;${edgeless_root}/functions/" \
    --single-trigger-wasm ${edgeless_root}/functions/single_trigger/single_trigger.wasm \
    --redis-url redis://127.0.0.1:6379/ \
    --dataset-path dataset/ \
    --append \
    --additional-fields "${additional_fields}" \
    --additional-header "${additional_header}"

echo "Stop EDGELESS cluster"
for node in ${nodes}
do
    ssh ${node} "killall edgeless_node_d"
done

killall edgeless_con_d
killall edgeless_orc_d
killall edgeless_bal_d
killall edgeless_node_d

sleep 3


done
done
done
