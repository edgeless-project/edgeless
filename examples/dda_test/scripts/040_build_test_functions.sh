#!/bin/bash
# SPDX-FileCopyrightText: © 2024 Siemens AG
# SPDX-License-Identifier: MIT

# Build all dda demo functions
dda_demo_functions="
dda_com_test 
dda_state_test 
dda_store_test
"
for dda_demo_function in $dda_demo_functions ; do
    rm -f ../../../functions/$dda_demo_function/*.wasm 2> /dev/null
    ../../../target/debug/edgeless_cli function build ../../../functions/$dda_demo_function/function.json
done
