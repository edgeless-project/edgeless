# EDGELESS APIs

Files in the top-level of src define traits for all interfaces between the
different components of edgeless (also known as inner traits).
On the other hand, the directory `outer/` contains traits that put together
the traits from top-level into composite traits.
The code structure is designed to make it easier to implement the traits
through different inter-process communication library/protocols.
At the moment, the only

> Every implementation has to implement the inner and outer traits.

Currently, the only _complete_ implementation of EDGELESS APIs uses
[Google's gRPC](https://grpc.io/).
A CoAP implementation is also provided, using
[coap-lite](https://github.com/martindisch/coap-lite), which implements the
essential EDGELESS nodes' APIs.

## Inner traits

| API                      | Offered by         | Used by            | Description                                                                                                                                                                                                                          |
| ------------------------ | ------------------ | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| DomainRegistrationAPI    | ε-CON              | ε-ORC              | Domain formation. The API allows an ε-ORC to register its domain with an ε-CON, also providing an aggregate view of the node's capabilities. The registration is periodically refreshed as keep-alive.                               |
| FunctionInstanceAPI      | ε-ORC/node         | ε-CON/ε-ORC        | Function instance lifecycle. management.                                                                                                                                                                                             |
| GuestAPIFunction         | container function | node               | The node uses this API to trigger events and lifecycle management methods on a container function                                                                                                                                    |
| GuestAPIHost             | node               | container function | The container function uses this API to interact with the node's container run-time                                                                                                                                                  |
| InvocationAPI            | function instances | dataplane          | Function invocation. Called to handle events at function instances .                                                                                                                                                                 |
| NodeManagementAPI        | node               | ε-ORC              | Node management. The ε-ORC uses this API to update the node's dataplane and reset the node to a clean state                                                                                                                          |
| NodeRegistrationAPI      | ε-ORC              | node               | Orchestration domain formation. The API allows a node to register with an ε-ORC, also providing its capabilities, the health status, and performance samples (if enabled). The registration is periodically refreshed as keep-alive. |
| ResourceConfigurationAPI | ε-ORC/node         | ε-CON/ε-ORC        | Resource instance lifecycle.                                                                                                                                                                                                         |
| WorkflowInstanceAPI      | ε-CON              | client             | Cluster management. The client uses this API to manage the lifecycle of workflows and to query aggregate information on orchestration domains.                                                                                       |

## Outer traits

| Composite trait name | Implemented by     | Inner traits                                                     |
| -------------------- | ------------------ | ---------------------------------------------------------------- |
| AgentAPI             | node               | FunctionInstanceAPI, NodeManagementAPI, ResourceConfigurationAPI |
| ContainerFunctionAPI | container function | GuestAPIFunction                                                 |
| ContainerRuntimeAPI  | node               | GuestAPIHost                                                     |
| ControllerAPI        | ε-CON              | WorkflowInstanceAPI                                              |
| DomainRegisterAPI    | ε-CON              | DomainRegistrationAPI                                            |
| NodeRegisterAPI      | ε-ORC              | NodeRegistrationAPI                                              |
| OrchestratorAPI      | ε-ORC              | FunctionInstanceAPI, ResourceConfigurationAPI                    |

Note:

- The FunctionInstanceAPI and ResourceConfigurationAPI have a template type,
  which specifies the function/resource identifier to be used. In the APIs
  offered by the node the type is `InstanceId`, which contains the node
  identifier and the _physical_ function/resource identifier, while in the APIs
  offered by the ε-ORC the type is `DomainManagedInstanceId`, which only has the
  _logical_ function/resource identifier.

## Details

### GuestAPIFunction / GuestAPIHost

- GuestAPIFunction - gets implemented by the concrete virtualization-technology
  function instance; acts as a gRPC server to the edgeless node's client, which
  allows it to manage the instance and interact with it; boot / init / cast /
  call / stop
- GuestAPIHost - implemented by the edgeless node; function instance connects to
  it as a client to perform actions like sending events or logging telemetry
  data

```
+----------------------------+                           +-----------------------------+
|     EDGELESS Node          |                           |     Function Instance       |
| (implements GuestAPIHost)  |                           |(implements GuestAPIFunction)|
+----------------------------+                           +-----------------------------+
           ^                                                             |
           |                                                             |
           |                  Init, Boot, Cast, Call, Stop               |
           |  -------------------------------------------------------->  |
           |                                                             |
           |                                                             v
           |                TelemetryLog, Cast, Call, Sync,              |
           |  <--------------------------------------------------------  |
           |                DelayedCast, CastRaw, CallRaw                |
           |                                                             |
           |                                                             |
+----------------------------+                           +-----------------------------+
|     gRPC Server            |                           |     gRPC Server             |
|  (GuestAPIHost interface)  |                           | (GuestAPIFunction interface)|
+----------------------------+                           +-----------------------------+
```

Legend:

- Function instance starts and runs a gRPC server for GuestAPIFunction.
- EDGELESS node connects to it to manage lifecycle and messaging.
- Function instance connects back to the node’s GuestAPIHost to trigger cross-function operations or telemetry.
