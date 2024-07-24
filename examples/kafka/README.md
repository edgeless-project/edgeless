# A STEP BY STEP TEST KAFKA RESOURCE WITH EDEGELESS NODE 

These instructions will guide you through setting up a testing environment with Kafka and Zookeeper using Docker, starting an Edgeless node, creating a workflow to produce messages to a Kafka topic, and verifying that messages are received correctly.

## Prerequisites

- Docker installed on your system
- Rust and Cargo installed to build and run Edgeless node
- Git to clone Edgeless repository

### STEP 1: tart kafka+zookeeper with Docker

1. Create a directory with Docker configuration files and move on it: 
    
    ```bash
    ./kafka_docker
    ```

2. In this directory, create the `docker-compose.yml` file

as the following:



```yml
[general]
node_id = "fda6ce79-46df-4f96-a0d2-456f720f606c"
agent_url = "http://127.0.0.1:7021"
agent_url_announced = ""
invocation_url = "http://127.0.0.1:7002"
invocation_url_announced = ""
metrics_url = "http://127.0.0.1:7003"
orchestrator_url = "http://127.0.0.1:7011"

[wasm_runtime]
enabled = true

[container_runtime]
enabled = false
guest_api_host_url = "http://127.0.0.1:7100"

[resources]
http_ingress_url = "http://127.0.0.1:7035"
http_ingress_provider = "http-ingress-1"
http_egress_provider = "http-egress-1"
file_log_provider = "file-log-1"
redis_provider = "redis-1"

[user_node_capabilities]
num_cpus = 11
model_name_cpu = ""
clock_freq_cpu = 0
num_cores = 1
mem_size = 0
labels = []
is_tee_running = false
has_tpm = false
```



we will use it to start kafka and zookeeper

3. Open a shell and start kafka and zookeeper with the command:

```bash
docker-compose up -d
```

### STEP 2: Create edgeless_node

1. Inside the directory `./target/debug` create the default configuration of the node:

```bash
./taget/debug -t node.toml
```

the configuration file is the following:



```toml
[general]
node_id = "fda6ce79-46df-4f96-a0d2-456f720f606c"
agent_url = "http://127.0.0.1:7021"
invocation_url = "http://127.0.0.1:7002"
metrics_url = "http://127.0.0.1:7003"

[resources]
kafka_provider = "kafka-1"
kafka_broker_url = "localhost:9092"

[wasm_runtime]
enabled = true

[container_runtime]
enabled = false
```



### STEP 3: Start info of edgeless_node

1. Open another shell and start the command:

```bash
RUST_LOG=info ./edgeless_node_d 
```

This command tells `env_logger` to log log messages at the info level and above, which are located in the `edgeless_node_d` folder.
The env_logger log levels range from less verbose to more verbose: error, warn, info, debug, trace.
It is convenient to see what is happening underneath.

### STEP 4: Start edgeless_node test

1. Open another shell and move to the folder containing the file `node.toml`:

```bash
./target/debug
```

2. Execute the command:

```bash
./edgeless_inabox -t 
```

This command starts the edgeless_inabox application with the `-t` option, that means 'test'.
We will find the time, node and controller inside, so they will all be launched.

3. If there are already running configuration files remove them:

```bash
rm -f *.toml
```

4. To check which files are running:

```bash
ls *.toml
```

### STEP 5: Start edgeless_node and produce message

1. In the same shell as above run the command:

```bash
./edgeless_inabox 
```

if everything works correctly, we run the code with this command.
And now we can produce our messages.

