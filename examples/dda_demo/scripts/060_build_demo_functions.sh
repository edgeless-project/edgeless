#!/bin/bash
# SPDX-FileCopyrightText: © 2024 Siemens AG
# SPDX-License-Identifier: MIT

# Build all dda demo functions
dda_demo_functions="check_temperature move_arm"
for dda_demo_function in $dda_demo_functions ; do
    ../../../target/debug/edgeless_cli function build ../../../functions/$dda_demo_function/function.json
done
