### Noop example

The example creates a chain of one function that does nothing except calling a log directive.

First build the WASM binary:

```
target/debug/edgeless_cli function build examples/noop/noop_function/function.json
```

Then you can start and stop the workflow with:

```
ID=$(target/debug/edgeless_cli workflow start examples/noop/workflow.json)
target/debug/edgeless_cli workflow stop $ID
```