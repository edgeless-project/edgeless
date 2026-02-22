# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT

FROM registry.scontain.com/sconecuratedimages/crosscompilers:ubuntu24.04-scone6.0.5 AS builder

WORKDIR /myapp

RUN apt-get update && apt-get install -y \
    git pkg-config libtss2-dev protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

ENV PKG_CONFIG_ALLOW_CROSS=1

RUN git clone https://github.com/edgeless-project/edgeless.git \
 && cd edgeless && git checkout 7412d3b^ && cd ..

COPY edgeless_container_function/src/container_function.rs \
     /myapp/edgeless/edgeless_container_function/src/container_function.rs

WORKDIR /myapp/edgeless/edgeless_container_function

# Building the NON-debug binary
RUN scone-cargo build -p edgeless_container_function --release --verbose

# Check if the binary has SCONE-specific symbols. If missing, the binary will run as a "native" app and ignore SCONE_CONFIG_ID.
RUN nm /myapp/edgeless/target/x86_64-scone-linux-musl/release/edgeless_container_function_d | grep -q scone_ || \
    (echo "ERROR: Binary is not SCONE-compiled!" && exit 1)


# ---- SIGNER AND RUNTIME STAGE ----
FROM registry.scontain.com/sconecuratedimages/crosscompilers:runtime-scone6.0.5 AS runner

WORKDIR /app

# Copy the debug enclave binary produced by the crosscompiler
COPY --from=builder \
  /myapp/edgeless/target/x86_64-scone-linux-musl/release/edgeless_container_function_d \
  /usr/local/bin/edgeless_container_function

# Create a production-signed (non-debug) enclave binary.
# CAUTION: If you ship a debug enclave ("*_d"), CAS will treat it as untrustworthy and refuse to release secrets/config.
# BuildKit secret "enclave_key" is mounted at /run/secrets/enclave_key FOR THIS STEP ONLY and is NOT stored in any image layer.
# Use && to ensure the build ABORTS if any step fails.
# We also check if the secret file actually exists before trying to use it.

RUN --mount=type=secret,id=enclave_key \
    set -e; \
    export SCONE_SIGNER_VERBOSE=1; \
    \
    if [ ! -f /run/secrets/enclave_key ]; then \
        echo "ERROR: enclave_key secret not found! Did you forget --secret? Use --secret id=enclave_key,src=path/to/key"; \
        exit 1; \
    fi; \
    \
    echo "== BEFORE SIGNING =="; \
    scone-signer info /usr/local/bin/edgeless_container_function | head -n 20; \
    \
    echo "== SIGNING ENCLAVE =="; \
    scone-signer sign --production --key /run/secrets/enclave_key /usr/local/bin/edgeless_container_function; \
    \
    echo "== AFTER SIGNING =="; \
    scone-signer info /usr/local/bin/edgeless_container_function > /tmp/sign_info; \
    head -n 40 /tmp/sign_info; \
    echo "--- Verification ---"; \
    # Check the Public Key Modulus (Ensures it was signed by YOUR key)
    grep -A 2 "Modulus:" /tmp/sign_info; \
    # Verify Debug is 0 (required for --production)
    grep "Debug:" /tmp/sign_info; \
    # Extract the Enclave Measurement (MRENCLAVE)
    grep "MRENCLAVE:" /tmp/sign_info; \
    # File hash for audit trail
    sha256sum /usr/local/bin/edgeless_container_function; \
    rm /tmp/sign_info

EXPOSE 7101

ENV RUST_LOG=info \
    SCONE_MODE=hw \
    SCONE_VERSION=1 \
    SCONE_LAS_ADDR=172.17.0.1:18766 \
    SCONE_CAS_ADDR=172.17.0.1:18765 \
    SCONE_CONFIG_ID=policy/edgeless_function

# NOTE: These addresses use the Docker host bridge IP (172.17.0.1). This assumes LAS/CAS ports are published on the host
# and reachable from the container via the default Docker bridge routing.

CMD ["edgeless_container_function", "--endpoint", "http://0.0.0.0:7101"]
