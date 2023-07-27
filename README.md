# Edgeless MVP Implementation

This repository contains a research prototype of the Edgeless platform.

*There currently are no guarantees on Stability and API-Stability!*

## How to build:

The Implementation relies on Rust and the ProtoBuf compiler.

The easiest way to get started is the devcontainer shipped as part of this repository. 

Build Steps:

Build host code / tools:
`cargo build`

Use the CLI to build a guest:
`cargo run --bin=edgeless_cli function build examples/ping_pong/ping/function.json`
`cargo run --bin=edgeless_cli function build examples/ping_pong/pong/function.json`

## How to run:

This section will be expanded upon later. To get the basic system running:

`RUST_LOG=info cargo run --bin=edgeless_node_d --features=inabox`

To deploy the example functions (in a separate shell):

`RUST_LOG=info cargo run --bin=edgeless_cli workflow start examples/ping_pong/workflow.json`

## How to create functions/workflows:

Please refer to [documentation/rust_functions.md](documentation/rust_functions.md) and [documentation/workflows.md](documentation/workflows.md).

## Repository Layout

Each of the main services is implemented as its own crate (library + binary wrapper) that can be found at the root of the project:

* `edgeless_node`:  Worker Node including the agent and function runtime.
    * Exposes the `AgentAPI` consisting of the `FunctionInstanceAPI`
    * Exposes the `InvocationAPI` (data plane)
    * Has the feature `inabox` to run a standalone Edgeless environment with preconfigured instances of all required services inside a single binary. This is the easiest way to spawn up a development instance of edgeless.
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

Finally, `examples` contains example guests, and `edgeless_conf` contains reference configurations for the services.

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