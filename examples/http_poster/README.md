### http-poster example

The example creates a workflow triggered once per second, via the `counter`
function, whose output is posted to a server listening to port 10000 on
localhost.

1. Build the `counter` WASM binary following the
[instructions](../../functions/README.md). 

2. Start a simple web server on localhost in a separate shell:

```shell
python3 examples/http_poster/simpler_server.py
```

3. Make sure there is an EDGELESS cluster configured (e.g., with
   `edgeless_inabox`)

4. Start the workflow with:

```
target/debug/edgeless_cli workflow start examples/http_poster/workflow.json
```

5. You should see the following output on the web server shell:

```shell
1
2
3
4
5
6
<...>
```