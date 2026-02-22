#!/bin/bash
# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT

#
# Script to set up SCONE CAS locally:
#  - pulls CAS image
#  - creates ./cas directory
#  - writes cas/cas-default-owner-config.toml and cas/cas.toml
#  - sources dsd.sh to detect SGX devices
#  - generates docker-cas-compose.yml (with cas-init)
#  - computes & saves CAS hash output (SCONE_HASH=1)
#  - mounts sgx_default_qcnl.conf if present next to this script
#  - detects host GIDs for sgx + sgx_prv and adds group_add with numeric IDs
#
# IMPORTANT:
#   source ./setup-cas.sh
#   SGX_SIM_MODE=1 source ./setup-cas.sh

set -euo pipefail
export CAS="registry.scontain.com/sconecuratedimages/services:cas"
#export CAS="registry.scontain.com/sconecuratedimages/services:cas.preprovisioned.scone.cloud"
#docker pull "$CAS"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
CAS_DIR="$SCRIPT_DIR/cas"

mkdir -p "$CAS_DIR"

# Avoid root-owned ./cas trap
if [[ ! -w "$CAS_DIR" ]]; then
  echo "ERROR: '$CAS_DIR' is not writable by $(id -un)."
  echo "Fix: sudo chown -R $USER:$USER '$CAS_DIR'"
  exit 1
fi

# --- Save CAS hash output ---
HASH_OUT="$SCRIPT_DIR/cas.mrenclave.out"
echo "Computing CAS enclave measurement (SCONE_HASH=1) ..."
docker run --rm -e SCONE_HASH=1 "$CAS" 2>&1 | tee "$HASH_OUT" >/dev/null
echo "Wrote: $HASH_OUT"

# Create CAS default owner config
cat > "$CAS_DIR/cas-default-owner-config.toml" <<'EOF'
[ias]
spid            = "00000000000000000000000000000000"
linkable_quotes = true
sp_key          = "00000000000000000000000000000000"

[dcap]
subscription_key = "00000000000000000000000000000000"
EOF

# Create CAS service config
cat > "$CAS_DIR/cas.toml" <<'EOF'
[api]
api_listen = "0.0.0.0:8081"
enclave_listen = "0.0.0.0:18765"

[database]
path = "/var/lib/cas/cas.db"
EOF

# Detect devices via dsd.sh
source "$SCRIPT_DIR/dsd.sh"

# Detect SGX group IDs on the host (numeric)
source "$SCRIPT_DIR/sgx-groups.sh"
sgx_detect_groups

# SIM vs HW
SCONE_MODE="HW"
DEVICES_YAML=""

if [[ -z "${SGX_ENCLAVE_DEV:-}" || "${SGX_SIM_MODE:-0}" == "1" ]]; then
  SCONE_MODE="SIM"
  echo "Configuring CAS for SIMULATION mode."
else
  DEVICES_YAML="    devices:
      - \"${SGX_ENCLAVE_DEV}:${SGX_ENCLAVE_DEV}\""
  if [[ -n "${SGX_PROVISION_DEV:-}" ]]; then
    DEVICES_YAML="${DEVICES_YAML}
      - \"${SGX_PROVISION_DEV}:${SGX_PROVISION_DEV}\""
  fi
fi

# group_add snippet only makes sense in HW mode
GROUP_ADD_YAML=""
if [[ "${SCONE_MODE}" == "HW" ]]; then
  GROUP_ADD_YAML="$(sgx_group_add_yaml || true)"
  if [[ -z "${GROUP_ADD_YAML}" ]]; then
    echo "WARN: Could not detect host groups 'sgx'/'sgx_prv'. CAS may not access SGX devices unless it runs as root."
  fi
fi

# Mount sgx_default_qcnl.conf if present
QCNL_VOL_LINE=""
if [[ -f "$SCRIPT_DIR/sgx_default_qcnl.conf" ]]; then
  QCNL_VOL_LINE="      - \"./sgx_default_qcnl.conf:/etc/sgx_default_qcnl.conf:ro\""
  echo "Found $SCRIPT_DIR/sgx_default_qcnl.conf -> will mount into CAS."
else
  echo "No $SCRIPT_DIR/sgx_default_qcnl.conf found -> not mounting QCNL config."
fi

# CAS image runs as uid=1000/gid=1000 (you verified this)
CAS_UID="1000"
CAS_GID="1000"

# Generate docker-compose for CAS (with cas-init)
cat > "$SCRIPT_DIR/docker-cas-compose.yml" <<EOF
version: "3.8"

services:
  cas-init:
    image: alpine:3.20
    container_name: cas-init
    user: "0:0"
    volumes:
      - "cas-data:/var/lib/cas"
    command: >
      sh -c "mkdir -p /var/lib/cas &&
             chown -R ${CAS_UID}:${CAS_GID} /var/lib/cas &&
             chmod -R u+rwX /var/lib/cas"
    restart: "no"

  cas:
    image: $CAS
    container_name: cas
    depends_on:
      - cas-init
    networks:
      - scone-net
    command: cas -c /etc/cas/cas.toml
    environment:
      - SCONE_LOG=info
      - SCONE_MODE=${SCONE_MODE}
      - SCONE_LAS_ADDR=las:18766
    privileged: true
EOF

# Append group_add stanza (if available)
if [[ -n "${GROUP_ADD_YAML}" ]]; then
  cat >> "$SCRIPT_DIR/docker-cas-compose.yml" <<EOF
${GROUP_ADD_YAML}
EOF
fi

# Append devices stanza (HW mode)
if [[ -n "${DEVICES_YAML}" ]]; then
  cat >> "$SCRIPT_DIR/docker-cas-compose.yml" <<EOF
${DEVICES_YAML}
EOF
fi

# Append volumes/ports/networks/volumes section
cat >> "$SCRIPT_DIR/docker-cas-compose.yml" <<EOF
    volumes:
      - "./cas:/etc/cas:ro"
$( [[ -n "$QCNL_VOL_LINE" ]] && echo "$QCNL_VOL_LINE" )
      - "cas-data:/var/lib/cas"
    restart: on-failure
    ports:
      - "8082:8081"
      - "18765:18765"

networks:
  scone-net:
    external: true

volumes:
  cas-data:
EOF

echo "Wrote: $SCRIPT_DIR/docker-cas-compose.yml"
echo "Wrote: $CAS_DIR/cas.toml"
echo "Wrote: $CAS_DIR/cas-default-owner-config.toml"
echo "SCONE_MODE=${SCONE_MODE}"
if [[ "${SCONE_MODE}" == "HW" ]]; then
  echo "Devices mapped: ${SGX_ENCLAVE_DEV:-} ${SGX_PROVISION_DEV:-}"
  [[ -n "${SGX_GID:-}" ]] && echo "Host sgx GID: ${SGX_GID}"
  [[ -n "${SGX_PRV_GID:-}" ]] && echo "Host sgx_prv GID: ${SGX_PRV_GID}"
fi

echo "Start reliably:"
echo "  docker compose -f $SCRIPT_DIR/docker-cas-compose.yml up -d cas-init"
echo "  docker compose -f $SCRIPT_DIR/docker-cas-compose.yml up -d cas"

