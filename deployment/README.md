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

The command above builds a cluster with:
- 1 Redis
- 1 ε-CON
- 1 ε-ORC, configured to use the Redis as proxy and metrics-collector
- 1 node with WebAssembly run-time
- 1 node with file-log resource provider

## Deploy a cluster with multiple nodes

Example with 5 nodes:

```bash
NUM_NODES=5 ./make-docker-compose.sh
docker-compose up
```

## Example

Deploy a cluster in one shell:

```bash
docker-compose up
```

In another shell start a workflow that periodically generate a message to be saved in a file log:

```bash
target/debug/edgeless_cli workflow start examples/file_log/workflow.json
```

You can check the log with the following command:

```bash
docker exec -it deployment-edgeless_node_file_log-1 tail -f my-local-file.log
```

