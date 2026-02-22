#!/usr/bin/env bash
# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT

set -euo pipefail

# Go to repo root (two levels up from examples/sgx_container)
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# CAUTION: only images whose name contains edgeless-sgx-function- will get the SGX device mapping
# see edgeless/edgeless_node/src/container_runner/docker_utils.rs
IMAGE_TAG="edgeless-sgx-function-rust:dev"

KEY_FILE="${SCRIPT_DIR}/enclave-key.pem"

echo "Building container function image: ${IMAGE_TAG}"
echo "Repo root: ${REPO_ROOT}"
echo "Signing key: ${KEY_FILE}"

if [[ ! -f "$KEY_FILE" ]]; then
  echo "ERROR: Signing key not found at: $KEY_FILE"
  echo "Generate it first with ./gen-enclave-key.sh)"
  exit 1
fi

DOCKER_BUILDKIT=1 docker build --progress=plain --no-cache \
  --secret id=enclave_key,src="$KEY_FILE" \
  -t "${IMAGE_TAG}" \
  -f "${REPO_ROOT}/examples/sgx_container/Dockerfile" \
  "${REPO_ROOT}" 2>&1 | tee "${SCRIPT_DIR}/build.log"

echo "Build complete."

