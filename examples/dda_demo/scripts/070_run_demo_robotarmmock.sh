#!/bin/bash
# SPDX-FileCopyrightText: © 2024 Siemens AG
# SPDX-License-Identifier: MIT

# start the workflow
UUID=$(../../../target/debug/edgeless_cli workflow start ../workflow.json)

echo "Workflow ID: $UUID created"
echo "Call '../../../target/debug/edgeless_cli workflow stop $UUID' to stop the workflow again"