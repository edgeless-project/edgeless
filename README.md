# Edgeless MVP Implementation

## How to build:

The Implementation relies on Rust (Nightly) and the ProtoBuf compiler.

The easiest way to get started is the devcontainer shipped as part of this repository. 

## How to run:

This section will be expanded upon later. To get the basic system running:

`RUST_LOG=info cargo run --bin=edgeless_node_d --features=inabox`

To deploy the example functions (in a separate shell):

`RUST_LOG=info cargo run --bin=edgeless_cli workflow start examples/ping_pong/workflow.json`

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
    * TODO: Implement
    * Binary: `edgeless_bal_d`

* `edgeless_cli`: CLI to interact with the e-Controller
    * Binary: `edgeless_cli`

In addition to the services/binaries, the repository contains the following libraries:

* `edgeless_api`: Crate defining all inter-service APIs
    * Contains GRPC implementations of those inter-service APIs

* `edgeless_function`: Crate defining the interfaces for the tier1 guests
    * Tier1 Guests: Rust Functions compiled to WASM
    * Interface to the functions relies on the WASM component model.