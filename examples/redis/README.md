### Redis example

The example creates a function that periodically updates a counter on Redis.

First build the WASM binary:

```
target/debug/edgeless_cli function build examples/redis/counter/function.json
```

Then you can start and stop the workflow with:

```
ID=$(target/debug/edgeless_cli workflow start examples/redis/workflow.json)
target/debug/edgeless_cli workflow stop $ID
```