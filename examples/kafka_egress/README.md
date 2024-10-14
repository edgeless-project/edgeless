### Kafka-egress example

#### Requirements

Deploying a node with a `kafka_egress` resource provider requires
the `rdkafka` feature at compile time, e.g.:

```shell
cargo build --features rdkafka
```

#### Example

The example creates a function that periodically updates a counter on the
topic `test` of an [Apache Kafka server](https://kafka.apache.org/).

1. Get the latest Kafka release and extract it in `$KAFKADIR` (see
   [instructions](https://kafka.apache.org/quickstart)).

2. _If you don't have a Kafka cluster already_: install
   [docker-compose](https://docs.docker.com/compose/) and run:

```shell
cd examples/kafka_egress/
docker-compose up -d
cd -
```
 
3. Create a topic called `test`:

```shell
$KAFKADIR/bin/kafka-topics.sh --create --bootstrap-server localhost:9092 --topic test
```

4. Build the `counter` WASM binary:

```shell
target/debug/edgeless_cli function build functions/counter/function.json
```

5. Create the default configuration files for all the executables:

```shell
target/debug/edgeless_cli -t cli.toml
target/debug/edgeless_inabox -t
```

6. Start EDGELESS-in-a-box:

```shell
target/debug/edgeless_inabox
```

7. In another shell, start the workflow with:

```shell
ID=$(target/debug/edgeless_cli workflow start examples/redis/workflow.json)
```

8. Check that the messages are being produced by the workflow:

```shell
$KAFKADIR/bin/kafka-console-consumer.sh --bootstrap-server localhost:9092 --topic test
```

9. Stop the workflow:

```shell
target/debug/edgeless_cli workflow stop $ID
```