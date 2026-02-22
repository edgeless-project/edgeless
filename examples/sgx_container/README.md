# EDGELESS SGX Container Function Example (SCONE)


This example demonstrates how to run **EDGELESS** functions inside
**Intel SGX–protected containers** using **SCONE** and the EDGELESS
**container runtime**.

It provides a minimal, reproducible setup that:
- Builds a **Rust-based function** as a container image
- Cross-compiles the function with **SCONE**
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

Access to the SCONE container registry is required in order to pull images.

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
        └── workflow.json
```

___
## Build and run

```bash
user@host:~/edgeless$ examples/sgx_container/build.sh
user@host:~/edgeless$ target/debug/edgeless_cli workflow start examples/sgx_container/workflow.json
```


---

## Notes
1. SCONE cross-compilation currently uses an **older EDGELESS commit** that does **not** depend on `aws-lc-sys`. Newer versions introduce a dependency on `aws-lc-sys`, a low-level
crate wrapping C and assembly code, which prevents successful cross-compilation. This does not refer to the local copy of your EDGELESS but to the commit brought in the build stage. 

2. A one-stage Dockerfile variant is provided -with the relevant workflow & build script- for testing purposes eg measure function instantiation times when images are much larger. The default two-stage build achieves up to **10× smaller images**, typically reducing image sizes from a few GB to hundreds of MB.

3. The image name *must* contain `edgeless-sgx-function- ` otherwise EDGELESS will not pass the sgx driver to docker. Go to [docker_utils.rs](https://github.com/edgeless-project/edgeless/blob/main/edgeless_node/src/container_runner/docker_utils.rs) and [container_devices.rs](https://github.com/edgeless-project/edgeless/blob/main/edgeless_node/src/container_runner/container_devices.rs)
