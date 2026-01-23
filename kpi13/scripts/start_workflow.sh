#!/bin/bash
CLI_DIR=../build
CLI_CONFIG=../cfg/cli.toml
WORKFLOW_DIR=../workflow.json
RUST_LOG=info $CLI_DIR/edgeless_cli -c $CLI_CONFIG workflow start $WORKFLOW_DIR