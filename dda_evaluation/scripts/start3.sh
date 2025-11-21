#!/bin/bash
cd ../data
rm *.csv
cd ..
cd binaries
WF_ID=$(RUST_LOG=info ./edgeless_cli workflow start ../workflows/workflow_chain_3.json)
echo "Started workflow with id: $WF_ID"
sleep 20
./edgeless_cli workflow stop $WF_ID