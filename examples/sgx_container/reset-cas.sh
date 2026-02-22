#!/usr/bin/env bash
# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT

set -euo pipefail

# Reset SCONE CAS containers + CAS data volume + generated local files for this repo.
# This is destructive: it removes CAS state (DB/keys), containers, and local artifacts.

CAS_CONTAINER="${CAS_CONTAINER:-cas}"
CAS_INIT_CONTAINER="${CAS_INIT_CONTAINER:-cas-init}"

# Your repo shows the volume name as sgx_container_cas-data (compose-style).
CAS_VOLUME="${CAS_VOLUME:-sgx_container_cas-data}"

# Local artifacts in the current directory you want removed
FILES_TO_REMOVE=(
  "cas"
  "cas.mrenclave.out"
  "docker-cas-compose.yml"
)

echo "Stopping/removing containers (ignore if missing): $CAS_CONTAINER $CAS_INIT_CONTAINER"
docker rm -f "$CAS_CONTAINER" >/dev/null 2>&1 || true
docker rm -f "$CAS_INIT_CONTAINER" >/dev/null 2>&1 || true

echo "Removing CAS data volume (ignore if missing): $CAS_VOLUME"
docker volume rm "$CAS_VOLUME" >/dev/null 2>&1 || true

echo "Removing local artifacts:"
for f in "${FILES_TO_REMOVE[@]}"; do
  if [[ -e "$f" ]]; then
    rm -rf -- "$f"
    echo "  removed: $f"
  else
    echo "  missing: $f"
  fi
done

echo "Done."
