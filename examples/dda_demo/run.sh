#!/bin/bash

# build both functions
# ../../target/debug/edgeless_cli function build check_temperature/function.json
# ../../target/debug/edgeless_cli function build move_arm/function.json

# start the workflow
../../target/debug/edgeless_cli -c ../../cli.toml workflow start workflow.json