### Ollama example

The example creates a workflow that queries an ollama server.

![](ollama.png)

Install [ollama](https://ollama.com/), e.g., by following the
[quick-start instructions](https://github.com/ollama/ollama/blob/main/README.md#quickstart).

Start the workflow with:

```shell
ID=$(target/debug/edgeless_cli workflow start examples/ollama/workflow.json)
target/debug/edgeless_cli workflow stop $ID
```

Stop the workflow with:

```shell
target/debug/edgeless_cli workflow stop $ID
```