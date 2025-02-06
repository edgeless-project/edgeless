# Compilation stage
FROM ghcr.io/edgeless-project/ci-image:latest AS builder

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
controller_url = "http://${CONTROLLER_HOST}:${CONTROLLER_PORT}"\n\
domain_register_url = "http://${DOMAIN_REGISTER_HOST}:${DOMAIN_REGISTER_PORT}"\n' > /usr/local/etc/controller-template.toml

# Replace the variables and run the application
ENTRYPOINT ["/bin/bash", "-c", "\
  export CONTROLLER_HOST=${CONTROLLER_HOST:-0.0.0.0} && \
  export CONTROLLER_PORT=${CONTROLLER_PORT:-7001} && \
  export DOMAIN_REGISTER_HOST=${DOMAIN_REGISTER_HOST:-0.0.0.0} && \
  export DOMAIN_REGISTER_PORT=${DOMAIN_REGISTER_PORT:-7002} && \
  envsubst < /usr/local/etc/controller-template.toml > /usr/local/etc/controller.toml && \
  RUST_LOG=info /usr/local/bin/edgeless_con_d --config-file /usr/local/etc/controller.toml"]