TEST KAFKA RESOURCES WITH EDEGELESS NODE

These instructions will guide you through setting up a testing environment with Kafka and Zookeeper using Docker, starting an Edgeless node, creating a workflow to produce messages to a Kafka topic, and verifying that messages are received correctly.

Prerequisites

- Docker installed on your system
- Rust and Cargo installed to compile and run the Edgeless node
- Git to clone the Edgeless repository

STEP 1: start kafka+zookeeper with Docker

- create a directory with Docker configuration file: 
    ./kafka_docker

- In this directory create a file docker-compose.yml 

such as the following:

version: '3'

services:
  zookeeper:
    image: wurstmeister/zookeeper
    container_name: zookeeper
    ports:
      - "2181:2181"

  kafka:
    image: wurstmeister/kafka
    container_name: kafka
    ports:
      - "9092:9092"
    environment:
      KAFKA_ADVERTISED_HOST_NAME: localhost
      KAFKA_ADVERSISED_PORT: 9092
      KAFKA_CREATE_TOPICS: "test-topic:1:1"
      KAFKA_AUTO_CREATE_TOPICS_ENABLE: "false"
      KAFKA_ZOOKEEPER_CONNECT: "zookeerper:2181"
    depends_on:
      - zookeeper

we will use it to start kafka and zookeeper

- Open a shell and start kafka and zookeeper with the command:

docker-compose up -d

STEP 2: start edgeless_node

- Inside the directory ./target/debug create the default configuration of the node:

./taget/debug -t config.toml

the configuration file is the following:

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

- In another shell start edgeless node:
cargo run --config config.toml