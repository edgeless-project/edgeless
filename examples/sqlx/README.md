### Redis example

The example creates a function that periodically updates a counter on Redis.

First, build the `counter` WASM binary following the [instructions](../../functions/README.md). 

Then you can start and stop the workflow with:

```
ID=$(target/debug/edgeless_cli workflow start examples/redis/workflow.json)
target/debug/edgeless_cli workflow stop $ID
```