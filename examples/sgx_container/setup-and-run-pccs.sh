#!/bin/bash
# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT

# Usage:
#   # Start PCCS (requires Intel PCS API key):
#   INTEL_API_KEY="YOUR_INTEL_PCS_API_KEY" ./setup-and-run-pccs.sh
#
#   # Start PCCS and also register this machine's platform using PCKIDRetrievalTool:
#   INTEL_API_KEY="YOUR_INTEL_PCS_API_KEY" REGISTER_PLATFORM=true ./setup-and-run-pccs.sh
#
# Notes:
#   - The script prints (and also writes to ./pccs-tokens.env) the raw ADMIN_TOKEN and USER_TOKEN.
#   - Use USER_TOKEN with PCKIDRetrievalTool and ADMIN_TOKEN for PCCS admin API calls.
#   - PCCS runs on https://localhost:8081 and typically uses a self-signed cert (curl uses -k).

set -euo pipefail

# 1) Check required environment variable
if [[ -z "${INTEL_API_KEY:-}" ]]; then
  echo "Error: INTEL_API_KEY is not set"
  exit 1
fi

# 2) Generate random tokens (raw passwords)
ADMIN_TOKEN=$(openssl rand -hex 16)
USER_TOKEN=$(openssl rand -hex 16)

# 3) Generate SHA-512 hashes for the PCCS config (PCCS expects hashes, not raw tokens)
ADMIN_HASH=$(echo -n "$ADMIN_TOKEN" | sha512sum | awk '{print $1}')
USER_HASH=$(echo -n "$USER_TOKEN" | sha512sum | awk '{print $1}')

PCCS_IMAGE="registry.scontain.com/sconecuratedimages/services:pccs"
PCCS_CONTAINER_NAME="pccs"
PCCS_PORT=8081
PCCS_DATA_VOLUME="pccs-data"
SCONE_NETWORK="scone-net"

echo "Pulling PCCS image: $PCCS_IMAGE"
docker pull "$PCCS_IMAGE"

# 4) Run PCCS container
echo "Starting PCCS on port $PCCS_PORT..."
docker run -d \
  --name "$PCCS_CONTAINER_NAME" \
  --network "$SCONE_NETWORK" \
  -p "${PCCS_PORT}:${PCCS_PORT}" \
  -e APIKEY="$INTEL_API_KEY" \
  -e PCCS_PORT="$PCCS_PORT" \
  -e ADMIN_TOKEN_HASH="$ADMIN_HASH" \
  -e USER_TOKEN_HASH="$USER_HASH" \
  -e PCCS_LOG_LEVEL=debug \
  -v "${PCCS_DATA_VOLUME}:/var/lib/pccs" \
  "$PCCS_IMAGE" >/dev/null

# Persist tokens for automation
umask 077
TOKENS_FILE="${TOKENS_FILE:-./pccs-tokens.env}"
cat > "$TOKENS_FILE" <<EOF
PCCS_URL=https://localhost:${PCCS_PORT}
ADMIN_TOKEN=${ADMIN_TOKEN}
USER_TOKEN=${USER_TOKEN}
EOF


# Check reachability (no token)
echo "Waiting for PCCS HTTPS to come up..."
reachable=false
last_code="000"
for i in {1..60}; do
  last_code="$(curl -k -s -o /dev/null -w "%{http_code}" "https://localhost:${PCCS_PORT}/" || true)"
  if [[ "$last_code" != "000" ]]; then
    reachable=true
    break
  fi
  sleep 1
done

if [[ "$reachable" != "true" ]]; then
  echo "ERROR: PCCS did not become reachable on https://localhost:${PCCS_PORT}/ (last_code=$last_code)"
  docker logs --tail 200 "$PCCS_CONTAINER_NAME" || true
  exit 1
fi

echo "PCCS reachable (HTTP $last_code)."

# Patch PCCS config (SCONE image does not auto-apply env vars to default.json)
echo "Patching PCCS config with API key and token hashes..."
docker exec -it "$PCCS_CONTAINER_NAME" sh -lc '
set -e
CFG=/opt/intel/sgx-dcap-pccs/config/default.json

sed -i \
  -e "s/\"ApiKey\"[[:space:]]*:[[:space:]]*\"[^\"]*\"/\"ApiKey\": \"${APIKEY}\"/" \
  -e "s/\"UserTokenHash\"[[:space:]]*:[[:space:]]*\"[^\"]*\"/\"UserTokenHash\" : \"${USER_TOKEN_HASH}\"/" \
  -e "s/\"AdminTokenHash\"[[:space:]]*:[[:space:]]*\"[^\"]*\"/\"AdminTokenHash\" : \"${ADMIN_TOKEN_HASH}\"/" \
  "$CFG"

echo "Patched lines:"
grep -nE "ApiKey|UserTokenHash|AdminTokenHash" "$CFG"
' >/dev/null

echo "Restarting PCCS to pick up config changes..."
docker restart "$PCCS_CONTAINER_NAME" >/dev/null

# Auth readiness (admin token) - only after patch+restart
echo "Verifying PCCS auth"
auth_ok=false
last_code=""
for i in {1..30}; do
  last_code="$(curl -k -s -o /dev/null -w "%{http_code}" \
    -H "admin-token: ${ADMIN_TOKEN}" \
    "https://localhost:${PCCS_PORT}/sgx/certification/v4/platforms" || true)"

  if [[ "$last_code" != "401" && "$last_code" != "000" ]]; then
    auth_ok=true
    break
  fi
  sleep 1
done

if [[ "$auth_ok" != "true" ]]; then
  echo "ERROR: PCCS auth check failed (last HTTP code: ${last_code:-})"
  echo "Dumping configured hashes in container:"
  docker exec -it "$PCCS_CONTAINER_NAME" sh -lc 'grep -nE "ApiKey|UserTokenHash|AdminTokenHash" /opt/intel/sgx-dcap-pccs/config/default.json' || true
  docker logs --tail 200 "$PCCS_CONTAINER_NAME" || true
  exit 1
fi

echo "---------------------------------------------------"
echo "PCCS is running with Authentication Enabled"
echo "---------------------------------------------------"
echo "Admin Token (Raw): $ADMIN_TOKEN"
echo "User Token  (Raw): $USER_TOKEN"
echo "Tokens saved to: $TOKENS_FILE"
echo "---------------------------------------------------"
echo "Test Admin Access with:"
echo "curl -k -H \"admin-token: $ADMIN_TOKEN\" https://localhost:${PCCS_PORT}/sgx/certification/v4/platforms"

# 5) Optionally register this platform in PCCS (recommended for local dev)
if [[ "${REGISTER_PLATFORM:-false}" == "true" ]]; then
  if ! command -v PCKIDRetrievalTool >/dev/null 2>&1; then
    echo "REGISTER_PLATFORM=true but PCKIDRetrievalTool not found in PATH."
    echo "Install it first or add it to PATH."
    exit 1
  fi

  echo "Registering platform with PCCS (this host)..."

  # Avoid sudo creating an unreadable /tmp file:
  OUT="${OUT:-$HOME/pckid_retrieval.csv}"

  sudo PCKIDRetrievalTool \
    -f "$OUT" \
    -url "https://localhost:${PCCS_PORT}" \
    -user_token "$USER_TOKEN" \
    -use_secure_cert false

  # Ensure current user can read it
  sudo chown "${SUDO_USER:-$USER}:${SUDO_USER:-$USER}" "$OUT" 2>/dev/null || true
  sudo chmod 0644 "$OUT" 2>/dev/null || true

  echo "Platform registration attempt complete. Output: $OUT"
  head -n 5 "$OUT" || true
fi
