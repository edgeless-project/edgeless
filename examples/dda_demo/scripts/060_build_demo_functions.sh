#!/bin/bash
# SPDX-FileCopyrightText: © 2024 Siemens AG
# SPDX-License-Identifier: MIT

# Delete old function wasm files
rm -f $(find ../functions -name "*.wasm")

# Build all dda demo functions
for i in $(find ../functions -name function.json) ; do ../../../target/debug/edgeless_cli function build $i ; done
