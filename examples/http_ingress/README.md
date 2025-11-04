### HTTP hello world example

The example creates a chain of one function that waits for POST commands
matching a given host addressed to the balancer HTTP end-point and replies with
a 200 OK.

First, build the `http_processor` WASM binary following the
[instructions](../../functions/README.md). 

Then you can start the workflow:

```shell
ID=$(target/debug/edgeless_cli workflow start examples/http_ingress/workflow.json)
```

and verify that it works with curl:

```shell
curl -H "Host: demo.edgeless.com" -XPOST http://127.0.0.1:7007/hello
```

You will receive the following output:

```shell
World
```

Terminate the workflow with:

```shell
target/debug/edgeless_cli workflow stop $ID
```

#### Asynchronous invocation of the next component

The `call()` invocation method is used by default to invoke the next component
in the workflow, via the `new_request` output channel.

To invoke asynchronously the next component, i.e., using `cast()` instead,
you can specify the `async` configuration flag as true.

Furthermore, the example above uses the hostname (`demo.edgeless.com`) and
method (`POST`) to match the incoming HTTP command to the right workflow.
An easier, and less error-prone, alternative is to use directly the workflow
identifier in the query of the URL invoked by the client.

For instance, crete a workflow with:

```shell
ID=$(target/debug/edgeless_cli workflow start examples/http_ingress/workflow.json)
```

Now `$ID` contains the workflow identifier, which is fact used above to
terminate the workflow via `edgeless_cli stop`:

```shell
echo $ID
```

returns (for example -- the UUID is random and _will be different_ for you):

```shell
223dbff3-4412-474b-9ed4-cebf037e86dc
```

The same ID can be used to dispatch the body of HTTP commands directly
to the next component of `http-ingress` in the workflow.

Running this command:

```shell
echo "hello world" | curl -d@- "http://127.0.0.1:7007/"
```

results in a failure, because the `http-ingress` resource provider cannot match
the existing workflow instance with the request received:

```shell
Not Found
```

Instead, this command succeeds:

```shell
echo "hello world" | curl -d@- "http://127.0.0.1:7007/?wf_id=$ID"
```

In fact, the content of the `file-log` output file specified in
[workflow-async.json](workflow-async.json) is populated with the message
provided by `curl`:

```shell
cat target/debug/out.log
```

Gives:

```shell
2025-11-04T14:30:23.088069+00:00 hello world
```

#### Load balancing

It is possible that multiple resource instances match an incoming HTTP
command.
If that's the case, the `http-ingress` resource provider selects the target
component at random among those matching the host, method, and workflow
identifier (if specified, all are optional).

For example, start the following two workflows:

```shell
target/debug/edgeless_cli workflow start examples/http_ingress/workflow-multi-1.json
target/debug/edgeless_cli workflow start examples/http_ingress/workflow-multi-2.json
```

This creates two workflows, each with a `file-log` resource instances writing
to a separate file, called `out-1.log` and `out-2.log`.

When sending multiple (e.g., 10 in the example) HTTP commands, they are
dispatched at random to one of the two workflows:

```shell
for (( i = 0 ; i < 10 ; i++ )) ; do echo "hello world #$i" | curl -d@- "http://127.0.0.1:7007/" ; done
```

Result:

```shell
% cat target/debug/out-1.log
2025-11-04T14:35:38.949883+00:00 hello world #0
2025-11-04T14:35:38.982369+00:00 hello world #3
2025-11-04T14:35:38.989832+00:00 hello world #4
2025-11-04T14:35:38.996288+00:00 hello world #5
2025-11-04T14:35:39.002988+00:00 hello world #6
2025-11-04T14:35:39.009426+00:00 hello world #7
% cat target/debug/out-2.log
2025-11-04T14:35:38.964154+00:00 hello world #1
2025-11-04T14:35:38.974193+00:00 hello world #2
2025-11-04T14:35:39.015686+00:00 hello world #8
2025-11-04T14:35:39.022110+00:00 hello world #9
```
