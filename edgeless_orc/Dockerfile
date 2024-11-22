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
WORKDIR /usr/src/edgeless_orc
RUN cargo build --release --bin edgeless_orc_d --verbose

# Execution stage
FROM debian:bookworm-slim
COPY --from=builder /usr/src/target/release/edgeless_orc_d /usr/local/bin/edgeless_orc_d

# Install necessary tools (gettext-base includes envsubst)
RUN apt-get update && apt-get install -y gettext-base && rm -rf /var/lib/apt/lists/*

# Create the template file in the temporary location
RUN echo '\
[general]\n\
domain_id = "${DOMAIN_ID}"\n\
orchestrator_url = "${ORCHESTRATOR_URL}"\n\
orchestrator_url_announced = "${ORCHESTRATOR_URL_ANNOUNCED}"\n\
agent_url = "${AGENT_URL}"\n\
agent_url_announced = "${AGENT_URL_ANNOUNCED}"\n\
invocation_url = "${INVOCATION_URL}"\n\
invocation_url_announced = "${INVOCATION_URL_ANNOUNCED}"\n\
\n\
[baseline]\n\
orchestration_strategy = "${ORCHESTRATION_STRATEGY}"\n\
keep_alive_interval_secs = ${KEEP_ALIVE_INTERVAL_SECS}\n\
\n\
[proxy]\n\
proxy_type = "${PROXY_TYPE}"\n\
redis_url = "${REDIS_URL}"\n\
\n\
[collector]\n\
collector_type = "${COLLECTOR_TYPE}"\n\
redis_url = "${COLLECTOR_REDIS_URL}"\n' > /usr/local/etc/orchestrator-template.toml

# Replace the variables and run the application
ENTRYPOINT ["/bin/bash", "-c", "\
  export DOMAIN_ID=${DOMAIN_ID:-domain-1} && \
  export ORCHESTRATOR_URL=${ORCHESTRATOR_URL:-http://127.0.0.1:7011} && \
  export ORCHESTRATOR_URL_ANNOUNCED=${ORCHESTRATOR_URL_ANNOUNCED:-} && \
  export AGENT_URL=${AGENT_URL:-http://127.0.0.1:7121} && \
  export AGENT_URL_ANNOUNCED=${AGENT_URL_ANNOUNCED:-} && \
  export INVOCATION_URL=${INVOCATION_URL:-http://127.0.0.1:7102} && \
  export INVOCATION_URL_ANNOUNCED=${INVOCATION_URL_ANNOUNCED:-} && \
  export ORCHESTRATION_STRATEGY=${ORCHESTRATION_STRATEGY:-Random} && \
  export KEEP_ALIVE_INTERVAL_SECS=${KEEP_ALIVE_INTERVAL_SECS:-2} && \
  export PROXY_TYPE=${PROXY_TYPE:-None} && \
  export REDIS_URL=${REDIS_URL:-} && \
  export COLLECTOR_TYPE=${COLLECTOR_TYPE:-None} && \
  export COLLECTOR_REDIS_URL=${COLLECTOR_REDIS_URL:-} && \
  envsubst < /usr/local/etc/orchestrator-template.toml > /usr/local/etc/orchestrator.toml && \
  RUST_LOG=info /usr/local/bin/edgeless_orc_d --config-file /usr/local/etc/orchestrator.toml"]