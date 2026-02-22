# EDGELESS SGX Container Function Example (SCONE)


This example demonstrates how to run **EDGELESS** functions inside
**Intel SGX–protected containers** using **SCONE** and the EDGELESS
**container runtime**.

It provides a minimal, reproducible setup that:
- Builds a **Rust-based function** as a container image
- Cross-compiles the function with **SCONE**
- Includes scripts to setup a local PCCS/DCAP service, SCONE's LAS and CAS
- Includes a workflow containing a **single function**

---

## Prerequisites

To reproduce this example, the following requirements must be met.

### 1. Local EDGELESS Build

You must build EDGELESS locally and configure it appropriately:

- Enable the **container runtime** in `node.toml`
- Set `guest_api_host_url` to an address **reachable from inside containers**
  - ❌ Do **not** use `127.0.0.1` or `0.0.0.0`

---

### 2. SCONE Infrastructure Access

Access to the SCONE container registry is required in order to pull the appropriate images.

1. Register at <https://gitlab.scontain.com>
2. Create a **Personal Access Token** with appropriate permissions
3. Log in to the SCONE registry:

```bash
docker login registry.scontain.com
# username: <your username>
# password: <your personal access token>
```
___
## Function Logic

The function logic must be implemented in:

```text
edgeless/edgeless_container_function/src/container_function.rs
```
Folder layout:

```text
~/edgeless/
├── edgeless_container_function/
│   └── src/container_function.rs <- Add your function logic here
├── edgeless_api/
├── Cargo.toml
├── Cargo.lock
└── examples/
    └── sgx_container/
        ├── Dockerfile <- Dockerfile lives here
        ├── build.sh
        ├── workflow.json
        ├──  ...
        └── modified_files <- files that were modified to run the example
```
### About modified files: 
- `container_devices.rs` goes to `edgeless_node/src/container_runner/` in order to choose the in-tree SGX driver

- `container_function.rs`: implements the function logic, see folder structure above.
___
## Building the function image

To build the image, first generate an enclave signing private RSA key for scone-signer:

```bash
./generate-enclave-signing-key.sh
```
Then use the provided build script:
```bash
./build.sh
```
___
## Setup SGX Provisioning Certificate Caching Service (PCCS)  

Use the provided script to start a PCCS Docker container, generate and save admin/user auth tokens, patch the PCCS config with your Intel API key and token hashes, verify the service is reachable, and, optionally register the host platform with PCKIDRetrievalTool.


```bash
./setup-and-run-pccs.sh
```

---

## Setup and run SCONE's Local Attestation Service (LAS)

LAS runs on the node. It helps SGX apps create attestation quotes and connects them to the attestation infrastructure (like PCCS), so the enclave can prove it’s genuine and running trusted code.

```bash
./setup-las.sh
docker compose -f docker-las-compose.yml up -d
```
___

## Setup and run SCONE's Configuration and Attestation Service (CAS)
The role of CAS is to verify enclave attestation and securely deliver the function’s secrets only to trusted enclaves.

### CAS setup and run

```bash
./setup-cas.sh
docker compose -f docker-cas-compose.yml up -d
```

This script sets up CAS locally by preparing the CAS config files, detecting SGX devices and host SGX-related group IDs, generating the Docker Compose setup (including cas-init), computing and saving the CAS hash (SCONE_HASH=1), and mounting sgx_default_qcnl.conf if it exists next to the script.

### SCONE CAS provisioning

CAS must first be provisioned, meaning it is initialized as a trusted service (with its own identity/keys) so it can securely manage secrets and attestation decisions.

```bash
./provision-cas.sh
```

The script provisions CAS in HW mode by detecting SGX devices/groups, asks whether to do attested provisioning, and then provisions cas in a Docker container using LAS and your provisioning token/key hash (optionally verifying CAS via CAS_MRENCLAVE).
___

### Policy upload to CAS

After provisioning, CAS is ready to accept policies (i.e. security configuration and rules for apps). CAS uses those policies to decide which enclaves are allowed to receive secrets based on enclave identity measurements like:

- MRENCLAVE: the exact enclave code hash (specific build)
- MRSIGNER: the signer identity (who signed the enclave), allowing trust across compatible versions signed by the same key


```bash
./attest-cas-upload_policy.sh
```

This script attests CAS (optionally pinned to a specific MRENCLAVE), uploads a policy/session file to it, and finally verifies that the policy was stored successfully. 

A dummy `policy.yml` file is provided. Also, in case you want to completely remove CAS by deleting the CAS container, remove the CAS data volume, and clean the generated local CAS files/artifacts, use:

```bash
./reset-cas.sh
```
___


## Notes
1. SCONE cross-compilation currently uses an **older EDGELESS commit** that does **not** depend on `aws-lc-sys`. Newer versions introduce a dependency on `aws-lc-sys`, a low-level
crate wrapping C and assembly code, which prevents successful cross-compilation. This does not refer to the local copy of your EDGELESS but to the commit brought in the build stage.

2. The image name *must* contain `edgeless-sgx-function- ` otherwise EDGELESS will not pass the sgx driver to docker. See [docker_utils.rs](https://github.com/edgeless-project/edgeless/blob/main/edgeless_node/src/container_runner/docker_utils.rs) and [container_devices.rs](https://github.com/edgeless-project/edgeless/blob/main/edgeless_node/src/container_runner/container_devices.rs)

