### HTTP hello world example

The example creates a chain of one function that waits for POST commands
matching a given host addressed to the balancer HTTP end-point and replies with
a 200 OK.

First build the WASM binary:

```
target/debug/edgeless_cli function build examples/http_ingress/processing_function/function.json
```

Then you can start the workflow:

```
target/debug/edgeless_cli workflow start examples/http_ingress/workflow.json
```

and verify that it works with curl:

```
curl -H "Host: demo.edgeless.com" -XPOST http://127.0.0.1:7035/hello
```