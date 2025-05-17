### Serverless example (advanced)

The example creates a workflow where a message is generated periodically by
the `counter` function, which increments the value at each new message,
and then fed to a function vs. resource, which doubles the incoming value
and finally sends the result to a `file-log` resource.

The two workflows are:

- `workflow-wasm.json`, which uses the `double` WebAssembly function
- `workflow-serverless.json`, which uses a serverless resource provider that 
  assumes that an [OpenFaaS](https://www.openfaas.com/) function is reachable
  on localhost at port 5000

The example is meant to highlight the differences between the use of
a function vs. resource to perform the same task.

An OpenFaaS function XXX

First, build the `counter` and `double` WASM binary following the
[instructions](../../functions/README.md). 

Then you can start and stop the workflow with:

```
ID=$(target/debug/edgeless_cli workflow start examples/file_log/workflow.json)
target/debug/edgeless_cli workflow stop $ID
```