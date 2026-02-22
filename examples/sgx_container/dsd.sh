#!/bin/bash
# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT

# Determine SGX device nodes. Usage: source ./dsd.sh

export SGX_ENCLAVE_DEV=""
export SGX_PROVISION_DEV=""

# Detect Enclave Device
if [[ -c /dev/sgx_enclave ]]; then
    export SGX_ENCLAVE_DEV="/dev/sgx_enclave"
elif [[ -c /dev/sgx/enclave ]]; then
    export SGX_ENCLAVE_DEV="/dev/sgx/enclave"
elif [[ -c /dev/isgx ]]; then
    export SGX_ENCLAVE_DEV="/dev/isgx"
fi

# Detect Provision Device
if [[ -c /dev/sgx_provision ]]; then
    export SGX_PROVISION_DEV="/dev/sgx_provision"
elif [[ -c /dev/sgx/provision ]]; then
    export SGX_PROVISION_DEV="/dev/sgx/provision"
fi
