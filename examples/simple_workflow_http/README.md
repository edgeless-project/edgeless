### Simple workflow with HTTP external source/sink

The example creates the following chain:

- an HTTP ingress that wait for an external source to POST a message whose body contains an integer
- a function `incr` that increments by 1 the received integer number in the payload
- a function `double` that doubles the received integer number in the payload
- an HTTP egress that sends the received message to an external sink

First, build the `http_read_number`, `http_write_number`, `double`, and `incr` WASM binaries following the [instructions](../../functions/README.md). 

Then, you can request the controller to start the workflow:

```bash
target/debug/edgeless_cli workflow start examples/simple_workflow_http/workflow.json
```

In a shell open a TCP socket at port 10000 that plays the role of the external sink:

```bash
nc -l 10000
```

and in another use curl to emulate an external source:

```bash
curl -v -H "Host: demo.edgeless-project.eu" http://127.0.0.1:7008/read_number -d 42
```

In the sink you will receive the number `86=(42+1)*2`.