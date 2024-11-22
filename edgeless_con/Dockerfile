# Compilation stage
FROM rust:1.82 AS builder

WORKDIR /usr/src

RUN apt-get update -y && apt-get install -y \
    git \
    build-essential \
    clang \
    curl \
    libssl-dev \
    protobuf-compiler \
    libprotobuf-dev \
    mold \
 && rm -rf /var/lib/apt/lists/*
RUN rustup component add rustfmt clippy \
    && rustup target add wasm32-unknown-unknown \
    && cargo install wasm-tools

COPY . .
WORKDIR /usr/src/edgeless_con
RUN cargo build --release --bin edgeless_con_d --verbose

# Execution stage
FROM debian:bookworm-slim
COPY --from=builder /usr/src/target/release/edgeless_con_d /usr/local/bin/edgeless_con_d

# Install necessary tools (gettext-base includes envsubst)
RUN apt-get update && apt-get install -y gettext-base && rm -rf /var/lib/apt/lists/*

# Create the template file in the temporary location
RUN echo '\
controller_url = "${CONTROLLER_URL}"\n\
orchestrators = [\n\
    { domain_id = "${DOMAIN_ID}", orchestrator_url="${ORCHESTRATOR_URL}" }\n\
]\n' > /usr/local/etc/controller-template.toml

# Replace the variables and run the application
ENTRYPOINT ["/bin/bash", "-c", "\
  export CONTROLLER_URL=${CONTROLLER_URL:-http://127.0.0.1:7001} && \
  export DOMAIN_ID=${DOMAIN_ID:-domain-1} && \
  export ORCHESTRATOR_URL=${ORCHESTRATOR_URL:-http://127.0.0.1:7011} && \
  envsubst < /usr/local/etc/controller-template.toml > /usr/local/etc/controller.toml && \
  RUST_LOG=info /usr/local/bin/edgeless_con_d --config-file /usr/local/etc/controller.toml"]