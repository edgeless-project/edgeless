# Examples

Here you can find a bunch of examples of workflows / functions written for the
Edgeless MVP platform. 

- `file_log`: shows how to use a resource that appends the function arguments to a file local to the node
- `http_egress`: shows the HTTP egress feature of the e-Balancer by periodically issuing a GET to an external server
- `http_ingress`: shows the HTTP ingress feature of the e-Balancer by waiting for POST commands, to which the function replies with an OK message with fixed body, see [tutorial](http_ingress/README.md)
- `noop`: minimal workflow with a single function that does nothing, which can be used as a template to create more interesting stuff, see [tutorial](noop/README.md)
- `ping_pong`: shows how functions can be combined in a chain and how to access a shared state, see [tutorial](ping_pong/README.md)
- `ping_pong_cast`: same as above, but uses CAST instead of CALL events
- `redis`: shows how to use a resource that updates values on a Redis server
- `simple_workflow_http`: shows function chaining with external HTTP source/sink
- `tutorial-01`: shows how to create a DAG of functions/resources

Before running the examples you must build the system, see [building instructions](../BUILDING.md).

You may run all the examples with a single command by using the following script:

```shell
scripts/run_all_examples.sh
```