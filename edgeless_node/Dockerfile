# Compilation stage
FROM ghcr.io/edgeless-project/ci-image:latest AS builder

COPY . .
WORKDIR /usr/src/edgeless_node
RUN cargo build --release --bin edgeless_node_d --verbose

# Execution stage
FROM debian:bookworm-slim
COPY --from=builder /target/release/edgeless_node_d /usr/local/bin/edgeless_node_d

# Install necessary tools (gettext-base includes envsubst)
RUN apt-get update && apt-get install -y gettext-base && rm -rf /var/lib/apt/lists/*

# Create the template file in the temporary location
RUN echo '\
[general]\n\
node_id = "${NODE_ID}"\n\
agent_url = "http://${AGENT_HOST}:${AGENT_PORT}"\n\
agent_url_announced = "${AGENT_URL_ANNOUNCED}"\n\
invocation_url = "http://${INVOCATION_HOST}:${INVOCATION_PORT}"\n\
invocation_url_announced = "${INVOCATION_URL_ANNOUNCED}"\n\
node_register_url = "${NODE_REGISTER_URL}"\n\
\n\
[telemetry]\n\
metrics_url = "http://${TELEMETRY_METRICS_HOST}:${TELEMETRY_METRICS_PORT}"\n\
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
http_ingress_url = "http://${HTTP_INGRESS_HOST}:${HTTP_INGRESS_PORT}"\n\
http_ingress_provider = "${HTTP_INGRESS_PROVIDER}"\n\
http_egress_provider = "${HTTP_EGRESS_PROVIDER}"\n\
file_log_provider = "${FILE_LOG_PROVIDER}"\n\
redis_provider = "${REDIS_PROVIDER}"\n\
dda_provider = "${DDA_PROVIDER}"\n\
kafka_egress_provider = "${KAFKA_EGRESS_PROVIDER}"\n\
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
  export AGENT_HOST=${AGENT_HOST:-0.0.0.0} && \
  export AGENT_PORT=${AGENT_PORT:-7005} && \
  export AGENT_URL_ANNOUNCED=${AGENT_URL_ANNOUNCED:-} && \
  export INVOCATION_HOST=${INVOCATION_HOST:-0.0.0.0} && \
  export INVOCATION_PORT=${INVOCATION_PORT:-7002} && \
  export INVOCATION_URL_ANNOUNCED=${INVOCATION_URL_ANNOUNCED:-} && \
  export NODE_REGISTER_URL=${NODE_REGISTER_URL:-} && \
  export TELEMETRY_METRICS_HOST=${TELEMETRY_METRICS_HOST:-0.0.0.0} && \
  export TELEMETRY_METRICS_PORT=${TELEMETRY_METRICS_PORT:-7003} && \
  export TELEMETRY_LOG_LEVEL=${TELEMETRY_LOG_LEVEL:-info} && \
  export TELEMETRY_PERFORMANCE_SAMPLES=${TELEMETRY_PERFORMANCE_SAMPLES:-true} && \
  export WASM_RUNTIME_ENABLED=${WASM_RUNTIME_ENABLED:-true} && \
  export CONTAINER_RUNTIME_ENABLED=${CONTAINER_RUNTIME_ENABLED:-false} && \
  export GUEST_API_HOST_URL=${GUEST_API_HOST_URL:-} && \
  export HTTP_INGRESS_HOST=${HTTP_INGRESS_HOST:-0.0.0.0} && \
  export HTTP_INGRESS_PORT=${HTTP_INGRESS_PORT:-7035} && \
  export HTTP_INGRESS_PROVIDER=${HTTP_INGRESS_PROVIDER:-http-ingress-1} && \
  export HTTP_EGRESS_PROVIDER=${HTTP_EGRESS_PROVIDER:-http-egress-1} && \
  export FILE_LOG_PROVIDER=${FILE_LOG_PROVIDER:-file-log-1} && \
  export REDIS_PROVIDER=${REDIS_PROVIDER:-redis-1} && \
  export DDA_PROVIDER=${DDA_PROVIDER:-dda-1} && \
  export KAFKA_EGRESS_PROVIDER=${KAFKA_EGRESS_PROVIDER:-} && \
  export NUM_CPUS=${NUM_CPUS:-} && \
  export MODEL_NAME_CPU=${MODEL_NAME_CPU:-} && \
  export CLOCK_FREQ_CPU=${CLOCK_FREQ_CPU:-} && \
  export NUM_CORES=${NUM_CORES:-} && \
  export MEM_SIZE=${MEM_SIZE:-} && \
  export LABELS=${LABELS:-} && \
  export IS_TEE_RUNNING=${IS_TEE_RUNNING:-false} && \
  export HAS_TPM=${HAS_TPM:-false} && \
  envsubst < /usr/local/etc/node-template.toml > /usr/local/etc/node.toml && \
  RUST_LOG=info /usr/local/bin/edgeless_node_d --config-file /usr/local/etc/node.toml"]