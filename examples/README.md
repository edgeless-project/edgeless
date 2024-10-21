# Examples

Here you can find examples of EDGELESS workflows using the functions shipped
with the repository in [functions](../functions/) and showcasing some of the key
features provided by the platform.

- `container`: shows how to deploy a mixed workflow consisting of a function instance running in WASM, another as a container, and a resource
- `esp32_resources`: shows how to use a `scd30-sensor` resource
- `file_log`: shows how to use a resource that appends the function arguments to a file local to the node
- `http_egress`: shows the HTTP egress feature of the e-Balancer by periodically issuing a GET to an external server
- `http_ingress`: shows the HTTP ingress feature of the e-Balancer by waiting for POST commands, to which the function replies with an OK message with fixed body, see [tutorial](http_ingress/README.md)
- `kafka_egress`: shows host to use a resource that streams messages to an Apache Kafka server
- `matrix_mul`: shows how to create a single function or a chain of three functions performing multiplication of two internal matrices to increase the CPU load
- `noop`: minimal workflow with a single function that does nothing, which can be used as a template to create more interesting stuff, see [tutorial](noop/README.md)
- `ollama`: workflow that lets you interact via curl (`http-ingress` resource) with an ollama server (`ollama` resource), saving the responses to a file (`file-log` resource)
- `ping_pong`: shows how functions can be combined in a chain and how to access a shared state, see [tutorial](ping_pong/README.md)
- `ping_pong_cast`: same as above, but uses CAST instead of CALL events
- `redis`: shows how to use a resource that updates values on a Redis server
- `simple_workflow_http`: shows function chaining with external HTTP source/sink
- `tutorial-01`: shows how to create a DAG of functions/resources
- `vector_mul`: shows how to create a single function or a chain of three functions performing multiplication of an internal matrix by an input vector

Before running the examples you must build the system, see [building instructions](../docs/building.md).

You may run all (well, most of) the examples with a single command by using
the following script:

```shell
scripts/run_all_examples.sh
```

Some examples require the installation of external components and, thus, are
not run by default with the command above.
If you want to run *all* the examples then run:

```shell
RUN_SPECIALIZED_WORKFLOWS=1 scripts/run_all_examples.sh
```