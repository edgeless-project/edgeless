#!/usr/bin/env bash
# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT

set -euo pipefail

# Attest CAS and upload a policy/session to it.
# Works with debug CAS using "only_for_testing" flags.

POLICY_FILE="${1:-policy.yaml}"
CAS_ADDR="${CAS_ADDR:-cas:8081}"
LAS_ADDR="${LAS_ADDR:-las:18766}"
SCONE_NET="${SCONE_NET:-scone-net}"
CLI_IMAGE="${CLI_IMAGE:-registry.scontain.com/sconecuratedimages/crosscompilers:ubuntu24.04-scone6.0.5}"
CAS_MRENCLAVE="${CAS_MRENCLAVE:-}"

if [[ ! -f "$POLICY_FILE" ]]; then
  echo "ERROR: Policy file not found: $POLICY_FILE" >&2
  exit 1
fi

echo "--- Configuration ---"
echo "CAS_ADDR:    $CAS_ADDR"
echo "LAS_ADDR:    $LAS_ADDR"
echo "SCONE_NET:   $SCONE_NET"
echo "POLICY_FILE: $POLICY_FILE"

if [[ -n "$CAS_MRENCLAVE" ]]; then
  echo "CAS_MRENCLAVE pinned: $CAS_MRENCLAVE"
else
  echo "CAS_MRENCLAVE not set -> using --only_for_testing-trust-any (debug CAS)"
fi
echo "----------------------"

# We mount $PWD to /side so the policy file is visible inside the CLI container.
# We mount ~/.cas to /root/.cas because the SCONE CLI stores its client identity + attestation cache there.
# NOTE: Without --user, this container runs as root and may create/overwrite root-owned files in ~/.cas on the host.
docker run --rm -it \
  --network "$SCONE_NET" \
  -v "$PWD:/side" \
  -v "$HOME/.cas:/root/.cas" \
  -w /side \
  -e "SCONE_CAS_ADDR=$CAS_ADDR" \
  -e "SCONE_LAS_ADDR=$LAS_ADDR" \
  -e "CAS_MRENCLAVE=$CAS_MRENCLAVE" \
  -e "POLICY_FILE=$POLICY_FILE" \
  "$CLI_IMAGE" \
  bash -lc '
    set -euo pipefail

    ATTEST_OPTS=(
      --only_for_testing-ignore-signer
      --only_for_testing-debug
      --accept-group-out-of-date
      --accept-configuration-needed
    )

    echo "Attesting CAS"
    if [[ -n "${CAS_MRENCLAVE:-}" ]]; then
      scone cas attest "${ATTEST_OPTS[@]}" --mrenclave "${CAS_MRENCLAVE}" "${SCONE_CAS_ADDR}"
    else
      scone cas attest "${ATTEST_OPTS[@]}" --only_for_testing-trust-any "${SCONE_CAS_ADDR}"
    fi

    echo "Creating session from policy"
    scone session create --cas="${SCONE_CAS_ADDR}" "/side/${POLICY_FILE}"

    echo "Verifying stored session in CAS"
    scone session read --cas="${SCONE_CAS_ADDR}" policy \
      | grep -nE "name:|version:|mrenclaves:|DEMO_SECRET|SCONE_ALLOW_DLOPEN|SCONE_LOG" || true

    echo "Success: Policy uploaded."
  '

