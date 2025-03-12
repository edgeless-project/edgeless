# EDGELESS command-line clients

There are two command-line clients that can be used to interact with
live EDGELESS components without developing your own gRPC client:

- `edgeless_cli`: the main command-line utility to build WebAssembly functions
  and query the ε-CON
- `proxy_cli`: a utility to interact with the ε-ORC via its _proxy_, if enabled

## edgeless_cli

The `edgeless_cli` has two main functions.

First, it allows to build WebAssembly functions by providing the function
specifications in a JSON format.

For example, to build the `noop` function:

```shell
target/debug/edgeless_cli function build functions/noop/function.json
```

This build mode does not require any live EDGELESS component.

Second, it allows interaction with a live ε-CON via the `WorkflowInstance` API.
The operations currently allowed are reported in the table below.

| Operation          | Argument                              | Description                                  |
| ------------------ | ------------------------------------- | -------------------------------------------- |
| `workflow start`   | Path of a JSON workflow specification | Create a new workflow                        |
| `workflow stop`    | Workflow identifier                   | Stop an active workflow                      |
| `workflow list`    |                                       | List the identifiers of the active workflows |
| `workflow inspect` | Workflow identifier                   | Show details about an active workflow        |
| `domain list`      |                                       | List the domain identifiers                  |
| `domain inspect`   | Domain identifier                     | Show details about an orchestration domain   |

## proxy_cli

`proxy_cli` requires that the ε-ORC has been configured with a proxy enabled.

Then, it allows to:

1. Query the status of the internal data structures of the ε-ORC, which are
   mirrored in the proxy.
2. Send local orchestration intents to the ε-ORC (see the delegated orchestrator
   concept in [this guide](local_orchestration.md)).
