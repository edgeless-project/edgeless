### Noop example

The example creates a chain of one function that does nothing except calling a log directive.

First, build the `noop` WASM binary following the [instructions](../../functions/README.md). 

Then you can start and stop the workflow with:

```
ID=$(target/debug/edgeless_cli workflow start examples/noop/workflow.json)
target/debug/edgeless_cli workflow stop $ID
```