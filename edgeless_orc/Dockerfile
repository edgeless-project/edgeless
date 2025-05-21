# Compilation stage
FROM ghcr.io/edgeless-project/building-image:latest AS builder

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
domain_register_url = "http://${DOMAIN_REGISTER_HOST}:${DOMAIN_REGISTER_PORT}"\n\
subscription_refresh_interval_sec = ${SUBSCRIPTION_REFRESH_INTERVAL_SEC}\n\
domain_id = "${DOMAIN_ID}"\n\
orchestrator_url = "http://${ORCHESTRATOR_HOST}:${ORCHESTRATOR_PORT}"\n\
orchestrator_url_announced = "${ORCHESTRATOR_URL_ANNOUNCED}"\n\
node_register_url = "http://${NODE_REGISTER_HOST}:${NODE_REGISTER_PORT}"\n\
\n\
[baseline]\n\
orchestration_strategy = "${ORCHESTRATION_STRATEGY}"\n\
\n\
[proxy]\n\
proxy_type = "${PROXY_TYPE}"\n\
redis_url = "${REDIS_URL}"\n\
\n\
[proxy.dataset_settings]\n\
dataset_path = "${DATASET_PATH}"\n\
append = ${APPEND}\n\
additional_fields = "${ADDITIONAL_FIELDS}"\n\
additional_header = "${ADDITIONAL_HEADER}"\n' > /usr/local/etc/orchestrator-template.toml

# Replace the variables and run the application
ENTRYPOINT ["/bin/bash", "-c", "\
  export DOMAIN_REGISTER_HOST=${DOMAIN_REGISTER_HOST:-} && \
  export DOMAIN_REGISTER_PORT=${DOMAIN_REGISTER_PORT:-7002} && \
  export SUBSCRIPTION_REFRESH_INTERVAL_SEC=${SUBSCRIPTION_REFRESH_INTERVAL_SEC:-2} && \
  export DOMAIN_ID=${DOMAIN_ID:-domain-1} && \
  export ORCHESTRATOR_HOST=${ORCHESTRATOR_HOST:-0.0.0.0} && \
  export ORCHESTRATOR_PORT=${ORCHESTRATOR_PORT:-7003} && \
  export ORCHESTRATOR_URL_ANNOUNCED=${ORCHESTRATOR_URL_ANNOUNCED:-} && \
  export NODE_REGISTER_HOST=${NODE_REGISTER_HOST:-0.0.0.0} && \
  export NODE_REGISTER_PORT=${NODE_REGISTER_PORT:-7004} && \
  export ORCHESTRATION_STRATEGY=${ORCHESTRATION_STRATEGY:-Random} && \
  export PROXY_TYPE=${PROXY_TYPE:-None} && \
  export REDIS_URL=${REDIS_URL:-} && \
  export DATASET_PATH=${DATASET_PATH:-} && \
  export APPEND=${APPEND:-true} && \
  export ADDITIONAL_FIELDS=${ADDITIONAL_FIELDS:-} && \
  export ADDITIONAL_HEADER=${ADDITIONAL_HEADER:-} && \
  envsubst < /usr/local/etc/orchestrator-template.toml > /usr/local/etc/orchestrator.toml && \
  RUST_LOG=info /usr/local/bin/edgeless_orc_d --config-file /usr/local/etc/orchestrator.toml"]