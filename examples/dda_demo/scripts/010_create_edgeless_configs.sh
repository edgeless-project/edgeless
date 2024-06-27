#!/bin/bash
# SPDX-FileCopyrightText: © 2024 Siemens AG
# SPDX-License-Identifier: MIT

# remove old config files
rm cli.toml
rm controller.toml
rm node.toml
rm orchestrator.toml

../../../target/debug/edgeless_con_d -t controller.toml
../../../target/debug/edgeless_orc_d -t orchestrator.toml
../../../target/debug/edgeless_node_d -t node.toml
../../../target/debug/edgeless_cli -t cli.toml