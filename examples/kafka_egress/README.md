### Kafka-egress example

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
 
3. Build the `counter` WASM binary following the
   [instructions](../../functions/README.md).
4. Create a topic called `test`:

```shell
$KAFKADIR/bin/kafka-topics.sh --create --bootstrap-server localhost:9092 --topic test
```

5. Start the workflow with:

```shell
ID=$(target/debug/edgeless_cli workflow start examples/redis/workflow.json)
```

6. Check that the messages are being produced by the workflow:

```shell
$KAFKADIR/bin/kafka-console-consumer.sh --bootstrap-server localhost:9092 --topic test
```

7. Stop the workflow:

```shell
target/debug/edgeless_cli workflow stop $ID
```