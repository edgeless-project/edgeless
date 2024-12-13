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