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

if [ ! -r docker-compose.yml ] ; then
    echo "file docker-compose.yml not found"
    exit 1
fi

OUT_FILE=docker-compose-$NUM_NODES.yml
head -n 51 docker-compose.yml > $OUT_FILE

# start the WASM nodes
for (( i = 1 ; i <= $NUM_NODES ; i++ )) ; 
do
  port1=$(( 10003 + (i - 1) * 2 ))
  port2=$(( 10004 + (i - 1) * 2 ))
  label=${LABELS[ $RANDOM % ${#LABELS[@]} ]}

  cat >> $OUT_FILE << EOF
  
  edgeless_node$i:
    image: edgeless_node:latest
    container_name: edgeless_node$i
    depends_on:
      - edgeless_orc
    environment:
      RUST_LOG: info
      SHOWCONF: 1
      INVOCATION_ENDPOINT: edgeless_node$i:$port1
      AGENT_ENDPOINT: edgeless_node$i:$port2
      NODE_REGISTER_URL: edgeless_orc:7004
      LABELS: '["$label"]'
      NUM_CORES: $(( $RANDOM % 10 ))
      NODE_TYPE: WASM
    ports:
      - $port1:$port1
      - $port2:$port2
EOF
done

# conditionally start DDA if an env variable is set to true. Also starts an mqtt
# broker for that purpose.
if [ "$DDA" == "true" ] ; then
  echo "adding DDA node"
  port1=11003
  port2=11004
  dda_port=12000
  label=${LABELS[ $RANDOM % ${#LABELS[@]} ]}

  cat >> $OUT_FILE << EOF
  
  edgeless_node_dda:
    image: edgeless_node:latest
    container_name: edgeless_node_dda
    depends_on:
      - edgeless_orc
    environment:
      RUST_LOG: info
      SHOWCONF: 1
      INVOCATION_ENDPOINT: edgeless_node_dda:$port1
      AGENT_ENDPOINT: edgeless_node_dda:$port2
      NODE_REGISTER_URL: edgeless_orc:7004
      DDA_ENDPOINT: dda:$dda_port
      LABELS: '["$label"]'
      NUM_CORES: $(( $RANDOM % 10 ))
      NODE_TYPE: DDA
    ports:
      - $port1:$port1
      - $port2:$port2
    
  dda:
    image: ghcr.io/coatyio/dda:latest
    volumes:
      - ./dda/:/dda 
    ports:
      - "12000:12000"
    
  mqtt_broker:
    image: eclipse-mosquitto:latest
    command: mosquitto -c /mosquitto-no-auth.conf
    ports:
      - "1883:1883"
EOF
fi
