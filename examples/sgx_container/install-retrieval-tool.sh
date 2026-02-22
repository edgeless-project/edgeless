#!/bin/bash
# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT

set -euo pipefail

# 1. Download and xtract to a system-wide path (/opt/intel)
echo "Downloading and extracting to /opt/intel"
curl -fsSLO https://download.01.org/intel-sgx/latest/linux-latest/distro/Debian12/sgx_debian_local_repo.tgz
sudo mkdir -p /opt/intel
sudo tar xzf sgx_debian_local_repo.tgz -C /opt/intel

# 2. Fix Permissions (rucial for Debian 12 _apt user)
echo "Fixing permissions"
sudo chown -R root:root /opt/intel/sgx_debian_local_repo
sudo find /opt/intel/sgx_debian_local_repo -type d -exec chmod 755 {} +
sudo find /opt/intel/sgx_debian_local_repo -type f -exec chmod 644 {} +

# 3. Setup Keyring
echo "Setting up keyring"
sudo mkdir -p /etc/apt/keyrings
sudo cp /opt/intel/sgx_debian_local_repo/keys/intel-sgx.key /etc/apt/keyrings/intel-sgx-keyring.asc

# 4. Create the correct local source list
echo "deb [signed-by=/etc/apt/keyrings/intel-sgx-keyring.asc arch=amd64] file:///opt/intel/sgx_debian_local_repo bookworm main" | \
    sudo tee /etc/apt/sources.list.d/sgx_debian_local_repo.list

# 5. Update and Install
sudo apt-get update
sudo apt-get install -y sgx-pck-id-retrieval-tool
