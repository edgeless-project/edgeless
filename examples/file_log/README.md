### FileLog example

The example creates a function that periodically send messages to be saved to a file.

First, build the `message_generator` WASM binary following the [instructions](../../functions/README.md). 

Then you can start and stop the workflow with:

```
ID=$(target/debug/edgeless_cli workflow start examples/file_log/workflow.json)
target/debug/edgeless_cli workflow stop $ID
```