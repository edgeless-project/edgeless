### Ollama example

The example creates a function that queries an ollama server.

XXX

Then you can start and stop the workflow with:

```
ID=$(target/debug/edgeless_cli workflow start examples/redis/workflow.json)
target/debug/edgeless_cli workflow stop $ID
```