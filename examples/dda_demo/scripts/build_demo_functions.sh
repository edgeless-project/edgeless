#!/bin/bash
# SPDX-FileCopyrightText: © 2024 Siemens AG
# SPDX-License-Identifier: MIT

# build both dda demo functions
../../../target/debug/edgeless_cli function build ../functions/check_temperature/function.json
../../../target/debug/edgeless_cli function build ../functions/move_arm/function.json
