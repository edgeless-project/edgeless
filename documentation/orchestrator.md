# EDGELESS orchestrator (ε-ORC)

|                        |                     |
| ---------------------- | ------------------- |
| Executable name        | `edgeless_orc_d`    |
| Default conf file name | `orchestrator.toml` |

The ε-ORC is the component that manages an EDGELESS orchestration domain.
It has the following main responsibilities:

1. **Domain formation**: it offers a `NodeRegistration` API through which
   nodes may register and notify their (static) capabilities and (dynamic)
   system information metrics. The nodes refresh periodically their
   registration with the ε-ORC, also indicating a deadline by which a lack of
   refresh should be considered as a failure.
2. **Domain orchestration**: the ε-CON assigns to the ε-ORC function and
   resource instances, which it is then responsible for handling at best
   within the nodes in its domain.
3. **Proxy maintenance**: the ε-ORC may optionally synchronize its internal
   data structures and performance metrics with an external database via
   a _Proxy_. The database can then be queried by third-party services for
   monitoring purposes or to implement the delegated orchestrator concept.

The ε-ORC subscribes to its ε-CON via the `DomainRegistration` API,
periodically refreshing the subscription (the period is configurable in the
configuration file).
When an ε-ORC first enters a cluster, its functions and resources are reset.

The ε-ORC has the following interfaces, also illustrated in the diagram below:

| Interface             | Configuration file URL |
| --------------------- | ---------------------- |
| FunctionInstance      | `orchestrator_url`     |
| ResourceConfiguration | `orchestrator_url`     |
| NodeRegistration      | `node_register_url`    |

![](diagrams-orc.png)

## Proxy

When used, the ε-ORC mirrors its internal data structures on the proxy.

Currently, we only support Redis, which is enabled by means of the following
section in `orchestrator.toml`: 

```ini
[proxy]
proxy_type = "Redis"
redis_url = "redis://127.0.0.1:6379"
```

_The Redis database is flushed automatically by the ε-ORC when it starts._

We provide a command-line interface, called `proxy_cli`, which can be used
as a convenient alternative to reading directly from the Redis database.

### Schema

| Key                                       | Value                                                                                                                                                                                                                        | Data type                              |
| ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| domain_info:domain_id                     | Identifier of the domain managed by this ε-ORC                                                                                                                                                                               | `String`                               |
| nodes:capabilities:`node_id`              | JSON object representing the capabilities of the node with given node identifier                                                                                                                                             | `NodeCapabilities`                     |
| node:health:`node_id`                     | JSON object representing the health status of the node with given node identifier                                                                                                                                            | `NodeHealthStatus`                     |
| performance:function_execution_time:`pid` | List of function execution times of the function with the given Physical Identifier, in fractional seconds, each associated with a timestamp with a millisecond resolution taken by the ε-ORC (format `exec_time,timestamp`) | `NodePerformanceSamples`               |
| provider:`provider_id`                    | JSON object representing the configuration of the resource provider with given identifier                                                                                                                                    | `ResourceProvider`                     |
| instance:`lid`                            | JSON object including the annotations of the function with given Logical identifier and the currently active instances (each with node identifier and physical function identifier)                                          | `ActiveInstance`                       |
| dependency:`lid`                          | JSON object representing the dependencies of the function with given Logical identifier through a map of output channel names to logical function identifiers                                                                | `HashMap<Uuid, HashMap<String, Uuid>>` |
|                                           |

The following identifiers are represented in
[UUID](https://en.wikipedia.org/wiki/Universally_unique_identifier) format:

- node's identifier (`node_id`)
- logical function/resource instance identifier (`lid`)
- physical function/resource instance identifier (`pid`)

The following identifiers are represented as free-text strings:

- domain identifier
- provider identifier