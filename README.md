# Edgeless MVP Implementation

This repository contains a research prototype of the Edgeless platform.

*There currently are no guarantees on Stability and API-Stability!*

## How to build:

The Implementation relies on Rust and the ProtoBuf compiler.

The easiest way to get started is the devcontainer shipped as part of this repository. 

Build Steps:

Build host code / tools:

```
cargo build
```

### NixOS

If using Nix / on NixOS then there is a simple [`flake.nix`](./flake.nix) that is invoked via the `direnv` [`.envrc`](./.envrc) to autoinstall Nix package dependencies and give you a bulid shell once you `direnv allow` in this directory.

## How to run:

It is recommended that you enable at least info-level log directives with:

```
export RUST_LOG=info
```

To get the basic system, first create default configuration files:

```
target/debug/edgeless_inabox -t 
target/debug/edgeless_cli -t cli.toml
```

that will create:

- `balancer.toml`
- `controller.toml`
- `node.toml`
- `orchestrator.toml`
- `cli.toml`

Then you can run the EDGELESS-in-a-box:

```
target/debug/edgeless_inabox
```

Congratulations, now a full EDGELESS system in running for you, though it is not doing much.
Below you will find two examples of workflows that can be created.

### Ping-pong examples

The example creates a chain of two functions: ping and pong. The ping function wakes up every 1 second and invokes the pong function, which merely terminates after replying.

First, you have to locally build the WASM binaries:

```
target/debug/edgeless_cli function build examples/ping_pong/ping/function.json
target/debug/edgeless_cli function build examples/ping_pong/pong/function.json
```

which will generate the files:

- `examples/ping_pong/ping/pinger.wasm`
- `examples/ping_pong/pong/ponger.wasm`

Then, you can request the controller to start the workflow:

```
ID=$(target/debug/edgeless_cli workflow start examples/ping_pong/workflow.json)
```

Now `$ID` contains the workflow identifier assigned by the controller.

You will observe on the logs that the pinger workflow is, indeed, invoked every 1 second. Furthermore, a counter is increased at every new invocation. This counter is the _state_ of the workflow, which is shared across multiple instances of this workflow and persists after their termination.

For example, if you stop the worfklow:

```
target/debug/edgeless_cli workflow stop $ID
```

and you start again the workflow later, you will see the counter resuming from the previous value (search for `{"count":NUM}` in the EDGELESS-in-a-box logs):

```
target/debug/edgeless_cli workflow start examples/ping_pong/workflow.json
```

You can always list the active workflows with:

```
target/debug/edgeless_cli workflow list
```

### HTTP hello world example

The example creates a chain of one function that waits for POST commands matching a given host addressed to the balancer HTTP end-point and replies with a 200 OK.

First build the WASM binary:

```
target/debug/edgeless_cli function build examples/http_ingress/processing_function/function.json
```

Then you can start the workflow:

```
target/debug/edgeless_cli workflow start examples/http_ingress/workflow.json
```

and verify that it works with curl:

```
curl -H "Host: demo.edgeless.com" -XPOST http://127.0.0.1:7035/hello
```

## How to create functions/workflows:

Please refer to [documentation/rust_functions.md](documentation/rust_functions.md) and [documentation/workflows.md](documentation/workflows.md).

## Repository Layout

Each of the main services is implemented as its own crate (library + binary wrapper) that can be found at the root of the project:

* `edgeless_node`:  Worker Node including the agent and function runtime.
    * Exposes the `AgentAPI` consisting of the `FunctionInstanceAPI`
    * Exposes the `InvocationAPI` (data plane)
    * Binary: `edgeless_node_d`

* `edgeless_orc`: e-Orchestrator
    * Exposes the `OrchestratorAPI` consisting of the `FunctionInstanceAPI`
    * Binary: `edgeless_orc_d`

* `edgeless_con`: e-Controller
    * Exposes the `ControllerAPI` consisting of the `WorkflowInstanceAPI`
    * Binary: `edgeless_con_d`

* `edgeless_bal`: e-Balancer
    * TODO: Implement Event Balancer
    * Contains the HTTP-Ingress and exposes a `ResourceConfigurationAPI` to configure it.
    * Binary: `edgeless_bal_d`

* `edgeless_cli`: CLI to interact with the e-Controller
    * Binary: `edgeless_cli`

* `edgeless_inabox`:a standalone EDGELESS environment with preconfigured instances of all required services inside a single binary. This is the easiest way to spawn up a development instance of edgeless

In addition to the services/binaries, the repository contains the following libraries:

* `edgeless_api`: Crate defining all inter-service APIs
    * Contains GRPC implementations of those inter-service APIs

* `edgeless_function`: Crate defining the interfaces for the tier1 guests
    * Tier1 Guests: Rust Functions compiled to WASM
    * Interface to the functions relies on the WASM component model.

* `edgeless_dataplane`: Crate defining the Edgeless dataplane.
    * Provides the primary communication chains
        * Used by communicating entities to send and receive events
    * Provides a local communication provider
    * Provides a remote communication provider based on the `InvocationAPI`

* `edgeless_http`: Crate containing HTTP-related types.
    * Specifies the interface between the Ingress and the functions consuming HTTP Events.

Finally, `examples` contains example guests.

## Contributing

This section contains some rules you should adhere to when contributing to this repository.

* Run the rust formatter before committing. This ensures we minimize the noise coming from, e.g., whitespace changes.
* Try to limit the number of warnings (ideally, there should not be any warnings). A good way to do this is to run `cargo fix` before running the formatter.
    *  Suggested workflow: `cargo fix --allow-staged --allow-dirty && cargo fmt && git commit`
* Do not introduce merge commits to the main branch. Merges to the main branch must be fast-forward.
    *   This can be achieved by rebasing your branch onto the main branch before merging.
* Add yourself to the list of contributors & adhere to the license.
    * Do not taint this repository with incompatible licenses!
    * Everything not MIT-licensed must be kept external to this repository.

## License

The Repository is licensed under the MIT License. Please refer to [LICENSE-MIT.txt](LICENSE-MIT.txt) and [CONTRIBUTORS.txt](CONTRIBUTORS.txt). 
