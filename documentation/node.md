# EDGELESS node

|                        |                   |
| ---------------------- | ----------------- |
| Executable name        | `edgeless_node_d` |
| Default conf file name | `node.toml`       |

The node is the component that hosts the function and resource instances,
thereby providing computing facilities (functions) and connecting EDGELESS
with external entities in the real world (resources).

Depending on the configuration, it can host multiple run-time environments
(such as WebAssembly and Docker containers) and resource providers, see
[basic concepts](basic_concepts.md) for more info on what is currently
supported.

All the nodes in an orchestration domain are interconnected to one another
via a full mesh of gRPC services creating a _data plane_ for the dispatch
of events in a synchronous or asynchronous manner, respectively via the
call and cast methods.

The node subscribes to its ε-ORC via the `NodeRegistration` API,
periodically refreshing the subscription (the period is configurable in the
configuration file).
When a node first enters a cluster, its functions and resources are reset.

The node has the following interfaces, also illustrated in the diagram below:

| Interface             | Configuration file URL |
| --------------------- | ---------------------- |
| FunctionInstance      | `agent_url`            |
| ResourceConfiguration | `agent_url`            |
| NodeManagement        | `agent_url`            |
| Invocation            | `invocation_url`       |

![](diagrams-node.png)

## Configuration

The node configuration file has the following sections.

- general:
  - node_id: UUID of the node, which identifies it uniquely within the cluster
  - agent_url/agent_url_announced: URL exposed by the node to the 
  - invocation_url/invocation_url_announced: URL of the node's local dataplane
  - node_register_url: URL of the ε-ORC, needed for registration
  - subscription_refresh_interval_sec: interval, in s, at which the node
    refreshes its registration with the ε-ORC, also providing it with health
    information and local telemetry samples
- telemetry: defines the local telemetry
- wasm_runtime: enable/disable and configure the WebAssembly run-time
- container_runtime: enable/disable and configure the container run-time
- resources: adds and configures the node's resources
- user_node_capabilities: defines the node's capabilities exposed to the ε-ORC;
  most of the capabilities are automatically detected (but can be overridden),
  expect the following:
  - labels: defines string labels attached with the node that can be used to
    guide the allocation of functions/resources to nodes by the ε-ORC depending
    of the workflow's annotations. One label is automatically set as
    `hostname:<hostname of the node>`
  - is_tee_running: Boolean that specifies if the node is running in a Trusted
    Execution Environment, such as Intel SGX
  - has_tpm: Boolean that specifies if the node has a Trusted Platform Module
    that is used for authenticated registration with the ε-ORC