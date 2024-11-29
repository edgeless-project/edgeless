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
WORKDIR /usr/src/edgeless_node
RUN cargo build --release --bin edgeless_node_d --verbose

# Execution stage
FROM debian:bookworm-slim
COPY --from=builder /usr/src/target/release/edgeless_node_d /usr/local/bin/edgeless_node_d

# Install necessary tools (gettext-base includes envsubst)
RUN apt-get update && apt-get install -y gettext-base && rm -rf /var/lib/apt/lists/*

# Create the template file in the temporary location
RUN echo '\
[general]\n\
node_id = "${NODE_ID}"\n\
agent_url = "http://0.0.0.0:${AGENT_PORT}"\n\
agent_url_announced = "${AGENT_URL_ANNOUNCED}"\n\
invocation_url = "http://0.0.0.0:${INVOCATION_PORT}"\n\
invocation_url_announced = "${INVOCATION_URL_ANNOUNCED}"\n\
metrics_url = "http://0.0.0.0:${METRICS_PORT}"\n\
orchestrator_url = "${ORCHESTRATOR_URL}"\n\
\n\
[telemetry]\n\
metrics_url = "http://0.0.0.0:${TELEMETRY_METRICS_PORT}"\n\
log_level = "${TELEMETRY_LOG_LEVEL}"\n\
performance_samples = ${TELEMETRY_PERFORMANCE_SAMPLES}\n\
\n\
[wasm_runtime]\n\
enabled = ${WASM_RUNTIME_ENABLED}\n\
\n\
[container_runtime]\n\
enabled = ${CONTAINER_RUNTIME_ENABLED}\n\
guest_api_host_url = "${GUEST_API_HOST_URL}"\n\
\n\
[resources]\n\
http_ingress_url = "http://0.0.0.0:${HTTP_INGRESS_PORT}"\n\
http_ingress_provider = "${HTTP_INGRESS_PROVIDER}"\n\
http_egress_provider = "${HTTP_EGRESS_PROVIDER}"\n\
file_log_provider = "${FILE_LOG_PROVIDER}"\n\
redis_provider = "${REDIS_PROVIDER}"\n\
\n\
[user_node_capabilities]\n\
num_cpus = ${NUM_CPUS}\n\
model_name_cpu = "${MODEL_NAME_CPU}"\n\
clock_freq_cpu = ${CLOCK_FREQ_CPU}\n\
num_cores = ${NUM_CORES}\n\
mem_size = ${MEM_SIZE}\n\
labels = [${LABELS}]\n\
is_tee_running = ${IS_TEE_RUNNING}\n\
has_tpm = ${HAS_TPM}\n' > /usr/local/etc/node-template.toml

# Replace the variables and run the application
ENTRYPOINT ["/bin/bash", "-c", "\
  export NODE_ID=${NODE_ID:-'fda6ce79-46df-4f96-a0d2-456f720f606c'} && \
  export AGENT_PORT=${AGENT_PORT:-7021} && \
  export AGENT_URL_ANNOUNCED=${AGENT_URL_ANNOUNCED:-} && \
  export INVOCATION_PORT=${INVOCATION_PORT:-7002} && \
  export INVOCATION_URL_ANNOUNCED=${INVOCATION_URL_ANNOUNCED:-} && \
  export METRICS_PORT=${METRICS_PORT:-7003} && \
  export ORCHESTRATOR_URL=${ORCHESTRATOR_URL:-} && \
  export TELEMETRY_METRICS_PORT=${TELEMETRY_METRICS_PORT:-7003} && \
  export TELEMETRY_LOG_LEVEL=${TELEMETRY_LOG_LEVEL:-info} && \
  export TELEMETRY_PERFORMANCE_SAMPLES=${TELEMETRY_PERFORMANCE_SAMPLES:-true} && \
  export WASM_RUNTIME_ENABLED=${WASM_RUNTIME_ENABLED:-true} && \
  export CONTAINER_RUNTIME_ENABLED=${CONTAINER_RUNTIME_ENABLED:-false} && \
  export GUEST_API_HOST_URL=${GUEST_API_HOST_URL:-} && \
  export HTTP_INGRESS_PORT=${HTTP_INGRESS_PORT:-7035} && \
  export HTTP_INGRESS_PROVIDER=${HTTP_INGRESS_PROVIDER:-http-ingress-1} && \
  export HTTP_EGRESS_PROVIDER=${HTTP_EGRESS_PROVIDER:-http-egress-1} && \
  export FILE_LOG_PROVIDER=${FILE_LOG_PROVIDER:-file-log-1} && \
  export REDIS_PROVIDER=${REDIS_PROVIDER:-redis-1} && \
  export NUM_CPUS=${NUM_CPUS:-40} && \
  export MODEL_NAME_CPU=${MODEL_NAME_CPU:-} && \
  export CLOCK_FREQ_CPU=${CLOCK_FREQ_CPU:-2971} && \
  export NUM_CORES=${NUM_CORES:-1} && \
  export MEM_SIZE=${MEM_SIZE:-0} && \
  export LABELS=${LABELS:-} && \
  export IS_TEE_RUNNING=${IS_TEE_RUNNING:-false} && \
  export HAS_TPM=${HAS_TPM:-false} && \
  envsubst < /usr/local/etc/node-template.toml > /usr/local/etc/node.toml && \
  RUST_LOG=info /usr/local/bin/edgeless_node_d --config-file /usr/local/etc/node.toml"]