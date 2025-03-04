#!/bin/bash

LABELS=("red" "blue" "green")

if [ "$NUM_NODES" == "" ] ; then
    echo "you must specify the number of nodes as NUM_NODES"
    exit 1
fi

if [[ $NUM_NODES -le 1 ]] ; then
    echo "NUM_NODES must be greater than 1"
    exit 1
fi

if [ ! -r template-docker-compose.yml ] ; then
    echo "file template-docker-compose.yml not found"
    exit 1
fi

if [ "$DDA" == "true" ] ; then
    echo "generated template will contain a dedicated DDA node"
    OUT_FILE=docker-compose-$NUM_NODES+dda.yml
else
    echo "generated template will not contain a dedicated DDA node"
    OUT_FILE=docker-compose-$NUM_NODES.yml
fi
# TODO: what's the length now
head -n 71 template-docker-compose.yml > $OUT_FILE

# start the WASM nodes
for (( i = 1 ; i <= $NUM_NODES ; i++ )) ; 
do
  port1=$(( 10003 + (i - 1) * 2 ))
  port2=$(( 10004 + (i - 1) * 2 ))
  label=${LABELS[ $RANDOM % ${#LABELS[@]} ]}
  uuid=$(cat /proc/sys/kernel/random/uuid)
  node_name=edgeless_node_$i

  cat >> $OUT_FILE << EOF

  edgeless_node$i:
    image: ghcr.io/edgeless-project/edgeless_node:v1.0.0
    container_name: $node_name
    depends_on:
      - edgeless_orc
    environment:
      RUST_LOG: info
      NODE_ID: $uuid
      AGENT_HOST: 0.0.0.0
      AGENT_PORT: 7005
      AGENT_URL_ANNOUNCED: http://$node_name:7005
      INVOCATION_HOST: 0.0.0.0
      INVOCATION_PORT: 7002
      INVOCATION_URL_ANNOUNCED: http://$node_name:7002
      NODE_REGISTER_URL: http://edgeless_orc:7004 
      SUBSCRIPTION_REFRESH_INTERVAL_SEC: 2
      TELEMETRY_METRICS_HOST: 0.0.0.0
      TELEMETRY_METRICS_PORT: 7003
      TELEMETRY_LOG_LEVEL: info
      TELEMETRY_PERFORMANCE_SAMPLES: true
      WASM_RUNTIME_ENABLED: true
      CONTAINER_RUNTIME_ENABLED: true
      NUM_CORES: $(( $RANDOM % 10 ))
EOF
done

# conditionally add a DDA if an env variable is set to true. Also add an mqtt
# broker for that purpose.
# TODO: adding dda requires the docker compose to be started from this dir, as
# the dda/config.toml is available only here
if [ "$DDA" == "true" ] ; then
  echo "adding a DDA node"
  port1=11003
  port2=11004
  label=${LABELS[ $RANDOM % ${#LABELS[@]} ]}
  uuid=$(cat /proc/sys/kernel/random/uuid)
  node_name=edgeless_node_dda

  cat >> $OUT_FILE << EOF
  
  edgeless_node_dda:
    image: ghcr.io/edgeless-project/edgeless_node:v1.0.0
    container_name: $node_name
    depends_on:
      - edgeless_orc
    environment:
      RUST_LOG: info
      NODE_ID: $uuid
      AGENT_HOST: 0.0.0.0
      AGENT_PORT: 7005
      AGENT_URL_ANNOUNCED: http://$node_name:7005
      INVOCATION_HOST: 0.0.0.0
      INVOCATION_PORT: 7002
      INVOCATION_URL_ANNOUNCED: http://$node_name:7002
      NODE_REGISTER_URL: http://edgeless_orc:7004 
      SUBSCRIPTION_REFRESH_INTERVAL_SEC: 2
      TELEMETRY_METRICS_HOST: 0.0.0.0
      TELEMETRY_METRICS_PORT: 7003
      TELEMETRY_LOG_LEVEL: info
      TELEMETRY_PERFORMANCE_SAMPLES: true
      WASM_RUNTIME_ENABLED: true
      CONTAINER_RUNTIME_ENABLED: true
      NUM_CORES: $(( $RANDOM % 10 ))
      DDA_PROVIDER: dda-1
    
  dda:
    image: ghcr.io/coatyio/dda:latest
    volumes:
      - ./dda/:/dda 
    
  mqtt_broker:
    image: eclipse-mosquitto:latest
    command: mosquitto -c /mosquitto-no-auth.conf
EOF
fi
