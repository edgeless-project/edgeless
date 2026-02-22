#!/usr/bin/env bash
# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT

set -euo pipefail

# Versions/URLs (from Intel DCAP "latest" Debian12 directory)
DCAP_BASE="https://download.01.org/intel-sgx/latest/dcap-latest/linux"
DEB12_DIR="$DCAP_BASE/distro/Debian12"
LOCAL_REPO_TGZ="sgx_debian_local_repo.tgz"
PCK_TGZ="PCKIDRetrievalTool_v1.24.100.2.tar.gz"
SHA_CFG="$DCAP_BASE/SHA256SUM_dcap_1.24.cfg"

# Install locations
INTEL_OPT="/opt/intel"
REPO_DIR="$INTEL_OPT/sgx_debian_local_repo"
KEYRING_DIR="/etc/apt/keyrings"
KEYRING_FILE="$KEYRING_DIR/intel-sgx-keyring.asc"
APT_LIST="/etc/apt/sources.list.d/sgx_debian_local_repo.list"

WORKDIR="${WORKDIR:-/tmp/intel-sgx-install}"
mkdir -p "$WORKDIR"
cd "$WORKDIR"

need_cmd() { command -v "$1" >/dev/null 2>&1; }
as_root() { if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then echo "Run as root (sudo)."; exit 1; fi; }

as_root

# Base deps
export DEBIAN_FRONTEND=noninteractive
apt-get update -y
apt-get install -y --no-install-recommends \
  ca-certificates curl gnupg tar gzip coreutils awk sed grep

download() {
  local url="$1" out="$2"
  if [[ -f "$out" ]]; then
    echo "Already have $out"
  else
    echo "Downloading $url"
    curl -fsSLo "$out" "$url"
  fi
}

verify_sha256_from_cfg() {
  local file="$1" cfg="$2" path_in_cfg="$3"
  local local_sum remote_sum
  local_sum="$(sha256sum "$file" | awk '{print $1}')"
  remote_sum="$(curl -fsSL "$cfg" | tr ' ' '\n' | paste - - | awk -v p="$path_in_cfg" '$2==p {print $1}')"

  if [[ -z "$remote_sum" ]]; then
    echo "ERROR: Could not find checksum entry for $path_in_cfg in $cfg"
    exit 1
  fi
  if [[ "$local_sum" != "$remote_sum" ]]; then
    echo "ERROR: Checksum mismatch for $file"
    echo " local:  $local_sum"
    echo " remote: $remote_sum"
    exit 1
  fi
  echo "Checksum OK for $file"
}

# 1) Local repo tgz
download "$DEB12_DIR/$LOCAL_REPO_TGZ" "$LOCAL_REPO_TGZ"
verify_sha256_from_cfg "$LOCAL_REPO_TGZ" "$SHA_CFG" "distro/Debian12/$LOCAL_REPO_TGZ"

mkdir -p "$INTEL_OPT"
# Re-extract cleanly each run to avoid stale repo content
rm -rf "$REPO_DIR"
tar xzf "$LOCAL_REPO_TGZ" -C "$INTEL_OPT"

# 2) Add APT source (Debian 12 = bookworm)
mkdir -p "$KEYRING_DIR"
cp -f "$REPO_DIR/keys/intel-sgx.key" "$KEYRING_FILE"

cat > "$APT_LIST" <<EOF
deb [signed-by=$KEYRING_FILE arch=amd64] file://$REPO_DIR bookworm main
EOF

apt-get update -y

# 3) Install SGX/DCAP runtime bits commonly needed for DCAP flows
# (These are exactly the "primary packages" Intel calls out for SGX application users.)
apt-get install -y \
  libsgx-quote-ex \
  libsgx-dcap-ql \
  sgx-aesm-service

systemctl enable --now sgx-aesm.service || true

# 4) Install PCKIDRetrievalTool tarball
download "$DEB12_DIR/$PCK_TGZ" "$PCK_TGZ"
verify_sha256_from_cfg "$PCK_TGZ" "$SHA_CFG" "distro/Debian12/$PCK_TGZ"

PCK_INSTALL_DIR="$INTEL_OPT/sgx-pck-id-retrieval-tool"
mkdir -p "$PCK_INSTALL_DIR"
tar xzf "$PCK_TGZ" -C "$PCK_INSTALL_DIR" --strip-components=1

# Try to locate the binary and link it into PATH
BIN_PATH="$(find "$PCK_INSTALL_DIR" -maxdepth 2 -type f -name 'PCKIDRetrievalTool' -perm -111 | head -n 1 || true)"
if [[ -z "$BIN_PATH" ]]; then
  echo "WARNING: Could not find an executable named PCKIDRetrievalTool under $PCK_INSTALL_DIR"
  echo "         List files with: find $PCK_INSTALL_DIR -maxdepth 3 -type f"
else
  ln -sf "$BIN_PATH" /usr/local/bin/PCKIDRetrievalTool
  echo "Installed: /usr/local/bin/PCKIDRetrievalTool -> $BIN_PATH"
fi

echo
echo "Done."
echo "Verify:"
echo "  systemctl status sgx-aesm.service --no-pager"
echo "  PCKIDRetrievalTool || true"
