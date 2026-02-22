#!/bin/bash
# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/dsd.sh"
source "$SCRIPT_DIR/sgx-groups.sh"
sgx_detect_groups

if [[ -z "${SGX_ENCLAVE_DEV:-}" ]]; then
  echo "Error: No SGX enclave device found. HW mode required."
  exit 1
fi

# --- CONFIG ---
CAS_ADDR="${CAS_ADDR:-cas:8081}"
CAS_PROVISIONING_TOKEN="${CAS_PROVISIONING_TOKEN:?set CAS_PROVISIONING_TOKEN}"
CAS_KEY_HASH="${CAS_KEY_HASH:?set CAS_KEY_HASH}"

echo "Using SGX enclave device: $SGX_ENCLAVE_DEV"
[[ -n "${SGX_PROVISION_DEV:-}" ]] && echo "Using SGX provision device: $SGX_PROVISION_DEV"
echo "Using CAS_ADDR: $CAS_ADDR"
echo "Using LAS via Docker DNS: las:18766"
[[ -n "${SGX_GID:-}" ]] && echo "Host sgx GID: ${SGX_GID}"
[[ -n "${SGX_PRV_GID:-}" ]] && echo "Host sgx_prv GID: ${SGX_PRV_GID}"

# Ask user about attestation
read -r -p "Do you want to attest CAS during provisioning? (y/n): " ATTEST
ATTEST="${ATTEST,,}"  # lowercase

PROVISION_MODE_ARGS=()
if [[ "$ATTEST" == "y" || "$ATTEST" == "yes" ]]; then
  if [[ -z "${CAS_MRENCLAVE:-}" ]]; then
    echo "CAS_MRENCLAVE is not set."
    echo "Use: get_mrenclave.sh <docker_cas_image> and export to the variable CAS_MRENCLAVE. Then exit."
    exit 1
  fi
  PROVISION_MODE_ARGS=(with-attestation --mrenclave "$CAS_MRENCLAVE")
else
  PROVISION_MODE_ARGS=(only_for_testing-without-attestation)
fi

# Build group-add args (may be empty)
GROUP_ARGS="$(sgx_group_add_docker_args || true)"

# shellcheck disable=SC2086
docker run --rm -it \
  --network scone-net \
  -v "$PWD:$PWD" -w "$PWD" \
  --device "$SGX_ENCLAVE_DEV" \
  ${SGX_PROVISION_DEV:+--device "$SGX_PROVISION_DEV"} \
  ${GROUP_ARGS} \
  -e SCONE_MODE=HW \
  -e SCONE_LAS_ADDR=las:18766 \
  registry.scontain.com/sconecuratedimages/crosscompilers:ubuntu24.04-scone6.0.5 \
  scone cas provision \
    --token "$CAS_PROVISIONING_TOKEN" \
    -c "$CAS_KEY_HASH" \
    --config-file ./cas/cas-default-owner-config.toml \
    "$CAS_ADDR" \
    "${PROVISION_MODE_ARGS[@]}"

echo "CAS provisioning complete."

