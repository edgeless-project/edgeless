### Redis example

The example creates a function that periodically updates a counter on Redis.

First, build the `counter` WASM binary following the [instructions](../../functions/README.md). 

Then you can start and stop the workflow with:

```shell
ID=$(target/debug/edgeless_cli workflow start examples/redis/workflow.json)
```

You can check the current value of the counter with:

```shell
redis-cli get example-redis
```

Stop the workflow with:

```shell
target/debug/edgeless_cli workflow stop $ID
```

## Per-workflow key

The workflow in `workflow-alt.json` is the same, but it automatically includes
the identifier of the workflow in the key.
Therefore, after starting the workflow with:

```shell
ID=$(target/debug/edgeless_cli workflow start examples/redis/workflow-alt.json)
```

You can check the current value of the counter with:

```shell
redis-cli get $ID:example-redis
```

## Redis resource as a state store

The workflow in `workflow-state.json` is similar to those above, but the value
of the counter is saved on the Redis server so that future instance will resume
counting from the last value saved.
Start the workflow with:

```shell
ID=$(target/debug/edgeless_cli workflow start examples/redis/workflow-state.json)
```

Check the current value of the counter with:

```shell
redis-cli get example-redis
```

Stop the workflow:

```shell
target/debug/edgeless_cli workflow stop $ID
```

When restarting the workflow, you can verify that the counter will resume
from the last value.