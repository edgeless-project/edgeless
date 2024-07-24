# A STEP BY STEP TEST KAFKA PRODUCER AND CONSUMER 

These instruction will guide you to use a producer and consumer application to find the latencies (i.e. time difference) between the timestamp of any message in `bench.prod` and the same message in `bench.cons`.

## Prerequisites

- Docker installed on your system
- Rust and Cargo installed to build and run our application
- Git to clone Edgeless repository

### STEP 1: Start kafka+zookeeper with Docker

1. Move on the directory with Docker configuration files: 
    
    ```bash
    ./kafka_docker
    ```

2. In this directory, we will find the `docker-compose.yml` file

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

### STEP 2: Start the consumer application

1. Move on the directory with the consumer application

```bash
cd examples/kafka/consumer
```

2. In this directory, we will find the `consumer.rs` file. 
Open another shell and run it with te command:

```bash
cargo run -- --broker localhost:9092 --topic test-topic --output bench.cons
```

3. We can modify the broker, topic and output name according to your needs 

### STEP 3: Start producer application

1. Move on the directory with the producer application

```bash
cd examples/kafka/producer
```

2. In this directory, we will find the `producer.rs` file. 
Open another shell and run it with te command:

```bash
cargo run -- --broker localhost:9092 --topic test-topic --duration 10 --period 1000 --output bench.prod
```

3. We can modify the broker, topic, duration, period and output name according to your needs 

### STEP 4: See the latencies of the messages

1. We will find `bench.prod` and `bench.cons` file in the directory producer and consumer. We need to open them and we can see the timestamp of the messages produced.

As the following for the producer:




```txt
1721831599.485653000 - Payload: 0, Partition: 0, Offset: 629
1721831599.585940000 - Payload: 1, Partition: 0, Offset: 630
1721831599.685719000 - Payload: 2, Partition: 0, Offset: 631
1721831599.786629000 - Payload: 3, Partition: 0, Offset: 632
1721831599.885959000 - Payload: 4, Partition: 0, Offset: 633
1721831599.985928000 - Payload: 5, Partition: 0, Offset: 634
1721831600.85791000 - Payload: 6, Partition: 0, Offset: 635
1721831600.185615000 - Payload: 7, Partition: 0, Offset: 636
1721831600.286436000 - Payload: 8, Partition: 0, Offset: 637
```



and for the consumer:




```txt
1721831599.508982000 - Payload: 0
1721831599.594104000 - Payload: 1
1721831599.693622000 - Payload: 2
1721831599.795294000 - Payload: 3
1721831599.894254000 - Payload: 4
1721831599.993596000 - Payload: 5
1721831600.92649000 - Payload: 6
1721831600.194013000 - Payload: 7
1721831600.295095000 - Payload: 8
```



2. We can compare the two timestamps of the same messages and calculate the latency