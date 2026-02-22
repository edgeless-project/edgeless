#!/bin/bash
# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT

# sgx-groups.sh
#
# WHY THIS IS USED:
# SGX device nodes (/dev/sgx_enclave, /dev/sgx_provision) are usually 0660 and owned by
# root:sgx / root:sgx_prv. Non-root containers need those group IDs to access SGX HW.
# Group names often don't exist inside containers, so we detect HOST numeric GIDs for
# sgx/sgx_prv and use them in compose `group_add` or `docker run --group-add`.

set -euo pipefail

sgx__get_gid() {
  local grp="$1"
  local line gid
  line="$(getent group "$grp" 2>/dev/null || true)"
  [[ -z "$line" ]] && return 1
  gid="$(echo "$line" | cut -d: -f3)"
  [[ -n "$gid" ]] || return 1
  echo "$gid"
}

sgx_detect_groups() {
  SGX_GID=""
  SGX_PRV_GID=""

  SGX_GID="$(sgx__get_gid sgx || true)"
  SGX_PRV_GID="$(sgx__get_gid sgx_prv || true)"

  export SGX_GID SGX_PRV_GID
}

sgx_group_add_yaml() {
  local out=""
  [[ -n "${SGX_GID:-}" ]] || [[ -n "${SGX_PRV_GID:-}" ]] || return 0

  out="    group_add:"
  [[ -n "${SGX_GID:-}" ]] && out="${out}
      - \"${SGX_GID}\"    # sgx"
  [[ -n "${SGX_PRV_GID:-}" ]] && out="${out}
      - \"${SGX_PRV_GID}\"   # sgx_prv"

  echo "$out"
}

sgx_group_add_docker_args() {
  local args=()
  [[ -n "${SGX_GID:-}" ]] && args+=(--group-add "${SGX_GID}")
  [[ -n "${SGX_PRV_GID:-}" ]] && args+=(--group-add "${SGX_PRV_GID}")
  printf '%s ' "${args[@]:-}"
}
