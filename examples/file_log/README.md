### FileLog example

The example creates a function that periodically send messages to be saved to a file.

First build the WASM binary:

```
target/debug/edgeless_cli function build examples/file_log/message_generator/function.json
```

Then you can start and stop the workflow with:

```
ID=$(target/debug/edgeless_cli workflow start examples/file_log/workflow.json)
target/debug/edgeless_cli workflow stop $ID
```