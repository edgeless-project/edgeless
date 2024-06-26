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

for (( i = 1 ; i <= $NUM_NODES ; i++ )) ; do

  port1=$(( 10003 + (i - 1) * 2 ))
  port2=$(( 10004 + (i - 1) * 2 ))
  label=${LABELS[ $RANDOM % ${#LABELS[@]} ]}

  cat >> $OUT_FILE << EOF
  
  edgeless_node$i:
    image: edgeless_node:latest
    depends_on:
      - edgeless_orc
    environment:
      RUST_LOG: info
      SHOWCONF: 1
      ORCHESTRATOR_ENDPOINT: edgeless_orc:7011
      INVOCATION_ENDPOINT: edgeless_node$i:$port1
      AGENT_ENDPOINT: edgeless_node$i:$port2
      LABELS: '["$label"]'
      NUM_CORES: $(( $RANDOM % 10 ))
      NODE_TYPE: WASM
    ports:
      - $port1:$port1
      - $port2:$port2
EOF

done

