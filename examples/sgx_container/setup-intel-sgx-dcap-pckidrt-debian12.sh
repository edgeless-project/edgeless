# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT

# Sets up Intel SGX/DCAP on Debian 12 using Intel's local APT repo (file://),
# installs AESM + DCAP runtime packages, and installs/links PCKIDRetrievalTool.
# Note: This does NOT run PCKIDRetrievalTool or configure PCCS/TLS.

#!/usr/bin/env bash
set -euo pipefail

# -------- Config (override via env if needed) --------
DCAP_BASE="${DCAP_BASE:-https://download.01.org/intel-sgx/latest/dcap-latest/linux}"
DEB12_DIR="${DEB12_DIR:-$DCAP_BASE/distro/Debian12}"

LOCAL_REPO_TGZ="${LOCAL_REPO_TGZ:-sgx_debian_local_repo.tgz}"
PCK_TGZ="${PCK_TGZ:-PCKIDRetrievalTool_v1.24.100.2.tar.gz}"

INTEL_OPT="${INTEL_OPT:-/opt/intel}"
REPO_DIR="$INTEL_OPT/sgx_debian_local_repo"
PCK_DIR="$INTEL_OPT/sgx-pckidrt"

KEYRING_DIR="${KEYRING_DIR:-/etc/apt/keyrings}"
KEYRING_FILE="$KEYRING_DIR/intel-sgx-keyring.asc"
APT_LIST="/etc/apt/sources.list.d/sgx_debian_local_repo.list"

WORKDIR="${WORKDIR:-/tmp/intel-sgx-dcap-setup}"
# -----------------------------------------------------

as_root() { [[ "${EUID:-$(id -u)}" -eq 0 ]] || { echo "Run as root (sudo)."; exit 1; }; }
need_cmd() { command -v "$1" >/dev/null 2>&1; }

as_root
need_cmd curl
need_cmd tar

echo "[1/6] Base deps"
export DEBIAN_FRONTEND=noninteractive
apt-get update -y
apt-get install -y --no-install-recommends ca-certificates curl gnupg tar gzip findutils coreutils

mkdir -p "$WORKDIR"
cd "$WORKDIR"

download() {
  local url="$1" out="$2"
  if [[ -f "$out" ]]; then
    echo "  - Already have $out"
  else
    echo "  - Downloading $url"
    curl -fsSLo "$out" "$url"
  fi
}

echo "[2/6] Remove old SGX local repo list(s)"
rm -f /etc/apt/sources.list.d/sgx_local_repo.list \
      /etc/apt/sources.list.d/sgx_debian_local_repo.list \
      /etc/apt/sources.list.d/*sgx*.list || true

echo "[3/6] Download + extract Intel SGX local repo into $INTEL_OPT"
download "$DEB12_DIR/$LOCAL_REPO_TGZ" "$LOCAL_REPO_TGZ"

mkdir -p "$INTEL_OPT"
rm -rf "$REPO_DIR"
tar xzf "$LOCAL_REPO_TGZ" -C "$INTEL_OPT"
chown -R root:root "$REPO_DIR"
chmod -R a+rX "$REPO_DIR"

# Sanity: ensure Release exists
if [[ ! -f "$REPO_DIR/dists/bookworm/Release" ]]; then
  echo "ERROR: $REPO_DIR/dists/bookworm/Release not found. Repo extract looks wrong."
  echo "Contents:"
  ls -la "$REPO_DIR" || true
  exit 1
fi

echo "[4/6] Add APT key + file:// repo source"
mkdir -p "$KEYRING_DIR"
cp -f "$REPO_DIR/keys/intel-sgx.key" "$KEYRING_FILE"
chmod 0644 "$KEYRING_FILE"

cat > "$APT_LIST" <<EOF
deb [arch=amd64 signed-by=$KEYRING_FILE] file://$REPO_DIR bookworm main
EOF

apt-get update -y

echo "[5/6] Install SGX/DCAP runtime packages"
apt-get install -y sgx-aesm-service libsgx-quote-ex libsgx-dcap-ql

echo "[6/6] Enable AESM service (unit name varies)"
systemctl daemon-reload

if systemctl list-unit-files | grep -q '^aesmd\.service'; then
  systemctl enable --now aesmd.service
  systemctl status aesmd.service --no-pager || true
elif systemctl list-unit-files | grep -q '^sgx-aesm\.service'; then
  systemctl enable --now sgx-aesm.service
  systemctl status sgx-aesm.service --no-pager || true
else
  echo "WARNING: Could not find aesmd.service or sgx-aesm.service."
  echo "Units matching sgx/aesm:"
  systemctl list-unit-files | grep -Ei 'sgx|aesm|aesmd' || true
fi

echo
echo "[extra] Install PCKIDRetrievalTool under $PCK_DIR and link to /usr/local/bin"
download "$DEB12_DIR/$PCK_TGZ" "$PCK_TGZ"

rm -rf "$PCK_DIR"
mkdir -p "$PCK_DIR"
tar xzf "$PCK_TGZ" -C "$PCK_DIR" --strip-components=1

BIN="$(find "$PCK_DIR" -maxdepth 2 -type f -name PCKIDRetrievalTool -perm -111 | head -n 1 || true)"
if [[ -z "$BIN" ]]; then
  echo "ERROR: Couldn't find PCKIDRetrievalTool executable under $PCK_DIR"
  find "$PCK_DIR" -maxdepth 3 -type f | head -n 50
  exit 1
fi

ln -sf "$BIN" /usr/local/bin/PCKIDRetrievalTool

echo
echo "Done."
echo "Verify:"
echo "  systemctl status aesmd.service --no-pager || systemctl status sgx-aesm.service --no-pager"
echo "  PCKIDRetrievalTool -help"
