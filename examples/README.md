# Examples

Here you can find a bunch of examples of workflows / functions written for the
Edgeless MVP platform. 

- `http_egress`: shows the HTTP egress feature of the e-Balancer by periodically issuing a GET to an external server
- `http_ingress`: shows the HTTP ingress feature of the e-Balancer by waiting for POST commands, to which the function replies with an OK message with fixed body, see [tutorial](http_ingress/README.md)
- `noop`: minimal workflow with a single function that does nothing, which can be used as a template to create more interesting stuff, see [tutorial](noop/README.md)
- `ping_pong`: shows how functions can be combined in a chain and how to access a shared state, see [tutorial](ping_pong/README.md)
- `ping_pong_cast`: same as above, but uses CAST instead of CALL events
- `simple_workflow_http`: shows function chaining with external HTTP source/sink