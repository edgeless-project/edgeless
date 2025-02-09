### Sqlx example

The example creates a function that create, update, delete and fetch state from
a centralized sql database.

First, build the `sqlx_test` WASM binary following the
[instructions](../../functions/README.md). 

Then you can start and stop the workflow with:

```
target/debug/edgeless_inabox
target/debug/edgeless_cli function build functions/sqlx_test/function.json
target/debug/edgeless_cli workflow start examples/slqx/workflow.json
target/debug/edgeless_cli workflow stop $ID
```

The function can be found in `functions/sqlx_test/function.json`.

Replace the metadata to a any information you like to insert in a json format.

The workflow id is managed by edgeless so please keep it as it is.
