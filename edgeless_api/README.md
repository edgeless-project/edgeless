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

## TLS and mTLS configuration

To protect the data on transit, TLS or mTLS can be activated optionally. For this, you will need to provide the configuration file  called `tls_config.toml` file on `target/debug`. This file allows for multiple configurations: from no TLS, to TLS (only verification of the server by the client), mTLS (mutual verification) or mTLS with a TPM, if the private key for the client was generated and is being used from inside of a TPM 2.0. In order to activate different configurations, comment or remove the lines that are irrelevant. The content expected in `tls_config.toml` is as follows:

```
[server]
server_cert_path = ""
server_key_path  = ""
server_ca_path   = "" #Comment for plain TLS
[client]
client_cert_path = "" #Comment for plain TLS
client_key_path  = "" #Comment for plain TLS
client_ca_path   = ""
domain_name      = "" #Comment for plain TLS
tpm_handle       = "" #Comment for plain TLS or for mTLS without TPM
```
You will always need to provide the server key and cert to activate TLS as well as the client ca, while the rest of the configuration is optional and depends on whether you want mTLS with or without the TPM.

**Important:** Due to limitations with Tonic, you will need to modify the endpoints being used accordingly to avoid the (m)TLS connection from failing:
  ⋅ If using TLS or mTLS, your endpoints will need to be `https`, otherwise Tonic will refuse to connect over an insecure HTTP2 connection.

  ⋅ If using mTLS with TPM, your endpoints will need to be `http`. This is needed because a custom TLS resolver based on rustls is being used to validate the challenge, instead of the Tonic default one. If `https` endpoints are kept, then Tonic will refuse to connect as it cannot decode the http2 content.

### Example configuration
The most secure setup consists of activating mTLS between the Orchestrator and Controller, and mTLS with the TPM with the Edge Node. For this, you will need to use all https:// endpoints between Orchestrator and Controller, except for "node_register_url" in the orchestrator which needs to be http:// for the custom mTLS. The Edge Node will need to use https:// except in `invocation_url`, `invocation_url_announced` and `node_register_url` to allow for mTLS with the TPM to work.

### Loading private keys onto the TPM
While mTLS is intended to be used with devices that contain a TPM and that have obtained their certificate following the Registered Authentication process, you can also generate or import your already generated keys onto the TPM to provide an extra layer of security. For this, you will need to use the `tpm2_tools` package and run the following commands:
```
tpm2_createprimary -Grsa2048:aes128cfb -C o -c parent.ctx
tpm2_import -C parent.ctx -G <KEY_TYPE:rsa/ecc> -i <YOUR_PRIV_KEY> -u client.pub -r client.priv
tpm2_load -C parent.ctx -u client.pub -r client.priv -c client.ctx
tpm2_evictcontrol -C o -c client.ctx 0x81010002
```

After this commands, your private key will be stored inside of the TPM and can be used for the mTLS.