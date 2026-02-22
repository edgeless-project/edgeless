#!/usr/bin/env bash
# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT

set -euo pipefail

# Generates an SGX/SCONE enclave signing key (RSA) for scone-signer
# SCONE/SGX Requirement: 3072 bits and Public Exponent = 3
KEY_FILE="${KEY_FILE:-enclave-key.pem}"
BITS="3072" # Fixed to SGX standard

command -v openssl >/dev/null 2>&1 || { echo "ERROR: openssl not found"; exit 1; }

# If the key already exists, don't overwrite it without warning
if [[ -f "$KEY_FILE" ]]; then
    echo "Warning: $KEY_FILE already exists. Backing up to ${KEY_FILE}.bak"
    mv "$KEY_FILE" "${KEY_FILE}.bak"
fi

umask 077
# The '-3' flag sets the public exponent to 3
openssl genrsa -out "$KEY_FILE" -3 "$BITS" >/dev/null 2>&1
chmod 600 "$KEY_FILE"

echo "Success: Wrote SGX-compatible key to $(pwd)/$KEY_FILE"
echo "--- Verification ---"
openssl rsa -in "$KEY_FILE" -text -noout | grep -E "Public-Key|Exponent"
