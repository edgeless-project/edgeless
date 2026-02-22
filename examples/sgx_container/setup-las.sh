#!/bin/bash
# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT

set -euo pipefail

# Defaults
: "${PCCS_CONTAINER_NAME:=pccs}"
: "${PCCS_PORT:=8081}"
AESM_SOCKET="/var/run/aesmd/aesm.socket"

# Set LAS and export it
export LAS="registry.scontain.com/sconecuratedimages/las:latest"

# Pull image
#docker pull "$LAS"

# Run detection
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/dsd.sh"

# Build devices stanza
DEVICES_YAML=""
if [[ -n "${SGX_ENCLAVE_DEV:-}" ]]; then
  DEVICES_YAML="    devices:
      - \"${SGX_ENCLAVE_DEV}:${SGX_ENCLAVE_DEV}\""
  if [[ -n "${SGX_PROVISION_DEV:-}" ]]; then
    DEVICES_YAML="${DEVICES_YAML}
      - \"${SGX_PROVISION_DEV}:${SGX_PROVISION_DEV}\""
  fi
fi

# Check for AESM socket on host
if [[ ! -S "$AESM_SOCKET" ]]; then
  echo "Warning: $AESM_SOCKET not found on host. LAS might fail to generate quotes." >&2
fi

# Create docker-compose file
cat > "$SCRIPT_DIR/docker-las-compose.yml" <<EOF
version: "3.8"

services:
  las:
    container_name: las
    image: $LAS
    networks:
      - scone-net
    environment:
      - SCONE_MODE=HW
      - SCONE_LOG=info
      - SCONE_QPL_DCAP_KEY=\${INTEL_API_KEY:?set INTEL_API_KEY}
      - SCONE_PCCS_URL=https://${PCCS_CONTAINER_NAME}:${PCCS_PORT}/sgx/certification/v4
      - SCONE_PCCS_USE_SECURE_CERT=false
    tty: true
    volumes:
      # This connects LAS to the host's healthy AESM service
      - "$AESM_SOCKET:$AESM_SOCKET"
EOF

# Append devices if available
if [[ -n "${DEVICES_YAML}" ]]; then
  cat >> "$SCRIPT_DIR/docker-las-compose.yml" <<EOF
${DEVICES_YAML}
EOF
fi

# Append the rest
cat >> "$SCRIPT_DIR/docker-las-compose.yml" <<'EOF'
    restart: on-failure
    ports:
      - "18766:18766"

networks:
  scone-net:
    external: true
EOF

echo "Wrote: $SCRIPT_DIR/docker-las-compose.yml"
echo "AESM socket mounted: $AESM_SOCKET"
