### Sqlx example

The example creates a function that create, update, delete and fetch state from a centralized sql database.

First, build the `sqlx_test` WASM binary following the [instructions](../../functions/README.md). 

Then you can start and stop the workflow with:

```
ID=$(target/debug/edgeless_cli workflow start examples/slqx/workflow.json)
target/debug/edgeless_cli workflow stop $ID
```