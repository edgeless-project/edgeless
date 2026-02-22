#!/bin/bash
# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT


LOG_FILE="build.log"

#Extracts MRENCLAVEs, clean up the output and get the last two unique values (Non-EDMM is listed first in the log)
MRENCLAVES=$(sed -n '/== AFTER SIGNING ==/,$p' "$LOG_FILE" | \
             grep "MRENCLAVE:" | \
             grep -oE '[a-f0-9]{64}' | \
             tail -n 2)

# Assign to variables
NON_EDMM=$(echo "$MRENCLAVES" | sed -n '1p')
EDMM=$(echo "$MRENCLAVES" | sed -n '2p')

echo "Function MRENCLAVE go into the session/policy YAML that will be uploaded to CAS"
echo ""
echo "mrenclave:"
echo "  - \"$EDMM\" # Use for SGX2 (Modern CPUs/Dynamic Memory Management)"
echo "  - \"$NON_EDMM\" # Use for SGX1 (Legacy CPUs/Static Memory Management)"
echo "You can use both to ensure your function runs on any SGX hardware."
