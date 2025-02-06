# Edgeless Deployment with Docker containers

This document provides instructions for deploying the Edgeless system components: Controller, Orchestrator and Node using Docker images. It includes guidance on setting up the necessary environment variables and accessing the container images from GitHub Packages.


## Overview

The Edgeless system consists of three primary components:

- **Controller**: Manages configurations and interacts with domains.
- **Orchestrator**: Manages orchestration across nodes.
- **Node**: Executes tasks and collects telemetry data.

## Container Images

Docker images for each Edgeless component are hosted on GitHub Packages. The following links provide access to the respective images:

- **Controller**: [edgeless_con](https://github.com/edgeless-project/edgeless/pkgs/container/edgeless_con)
- **Orchestrator**: [edgeless_orc](https://github.com/edgeless-project/edgeless/pkgs/container/edgeless_orc)
- **Node**: [edgeless_node](https://github.com/edgeless-project/edgeless/pkgs/container/edgeless_node)

## Environment Variables

Each component requires specific environment variables for configuration. Default values are provided, but these can be overridden as needed.

### Controller

- `CONTROLLER_HOST`: Default `0.0.0.0`
- `CONTROLLER_PORT`: Default `7001`
- `DOMAIN_REGISTER_HOST`: Default `0.0.0.0`
- `DOMAIN_REGISTER_PORT`: Default `7002`

### Orchestrator

- `DOMAIN_REGISTER_HOST`: Default `-`
- `DOMAIN_REGISTER_PORT`: Default `7002`
- `SUBSCRIPTION_REFRESH_INTERVAL_SEC`: Default `2`
- `DOMAIN_ID`: Default `domain-1`
- `ORCHESTRATOR_HOST`: Default `0.0.0.0`
- `ORCHESTRATOR_PORT`: Default `7003`
- `ORCHESTRATOR_URL_ANNOUNCED`: No default
- `NODE_REGISTER_HOST`: Default `0.0.0.0`
- `NODE_REGISTER_PORT`: Default `7004`
- `ORCHESTRATION_STRATEGY`: Default `Random`
- `PROXY_TYPE`: Default `None`
- `REDIS_URL`, `DATASET_PATH`, `APPEND`, `ADDITIONAL_FIELDS`, `ADDITIONAL_HEADER`: No defaults

### Node

- `NODE_ID`: Default `'fda6ce79-46df-4f96-a0d2-456f720f606c'`
- `AGENT_HOST`: Default `0.0.0.0`
- `AGENT_PORT`: Default `7005`
- `AGENT_URL_ANNOUNCED`: No default
- `INVOCATION_HOST`: Default `0.0.0.0`
- `INVOCATION_PORT`: Default `7002`
- `INVOCATION_URL_ANNOUNCED`: No default
- `NODE_REGISTER_URL`: No default
- `TELEMETRY_METRICS_HOST`: Default `0.0.0.0`
- `TELEMETRY_METRICS_PORT`: Default `7003`
- `TELEMETRY_LOG_LEVEL`: Default `info`
- `TELEMETRY_PERFORMANCE_SAMPLES`: Default `true`
- `WASM_RUNTIME_ENABLED`: Default `true`
- `CONTAINER_RUNTIME_ENABLED`: Default `false`
- `GUEST_API_HOST_URL`, `LABELS`, `KAFKA_EGRESS_PROVIDER`: No defaults

## Launching Containers

To deploy the Edgeless components (Controller, Orchestrator, and Node) and configure them to communicate with each other with a minimun configuration, follow the steps below. 

### Step 1: Create a Docker Network

First, create a Docker network to enable communication between the components:

```bash
docker network create edgeless-network
```

### Step 2: Deploy the Controller

Run the Edgeless Controller within the created network:

```bash
docker run --name edgeless_con --network edgeless-network ghcr.io/edgeless-project/edgeless_con:v1.0.0
```

### Step 3: Deploy the Orchestrator

Deploy the Orchestrator, ensuring it connects to the Controller using the network:

```bash
docker run --name edgeless_orc --network edgeless-network  -e DOMAIN_REGISTER_HOST=edgeless_con ghcr.io/edgeless-project/edgeless_orc:v1.0.0
```

### Step 4: Deploy a Node

Finally, deploy a Node, configuring it to communicate with the Orchestrator, and itself for agent communication:

```bash
docker run --name edgeless_node_1 --network edgeless-network \
  -e NODE_ID=node1 \
  -e AGENT_URL_ANNOUNCED=edgeless_node:7005 \
  -e NODE_REGISTER_URL=edgeless_orc:7002 \
  ghcr.io/edgeless-project/edgeless_node:v1.0.0
```