# Deployment scripts

## Build container images

```bash
docker build deployment/node -t edgeless_node
docker build deployment/controller -t edgeless_con
docker build deployment/orchestrator -t edgeless_orc
```

## Deploy a cluster with docker-compose

```bash
docker-compose up
```