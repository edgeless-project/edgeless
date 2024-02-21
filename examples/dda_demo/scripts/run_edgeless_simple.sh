#!/bin/bash
# SPDX-FileCopyrightText: © 2024 Siemens AG
# SPDX-License-Identifier: MIT

RUST_LOG=info ../../../target/debug/edgeless_con_d&
sleep 2
RUST_LOG=info ../../../target/debug/edgeless_orc_d&
sleep 2
RUST_LOG=info ../../../target/debug/edgeless_node_d