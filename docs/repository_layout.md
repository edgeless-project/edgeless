# EDGELESS repository layout

Each of the primary services is implemented as its own crate: library + (optional) binary wrapper.

Refer to the `README` file of a particular component to find out more.

You may want to check the [conventions of Rust's module system](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html).

## Core Components

These represent the core components of the EDGELESS project, as proposed in the
design phase.

### edgeless_node

Worker node including the agent and the function runtime.
  * Exposes the `AgentAPI` consisting of the `FunctionInstanceAPI`
  * Exposes the `InvocationAPI` (data plane)
  * Binary: `edgeless_node_d`

### edgeless_orc

ε-ORC (EDGELESS orchestrator)

  * Exposes the `OrchestratorAPI` consisting of the `FunctionInstanceAPI`
  * Binary: `edgeless_orc_d`

### edgeless_con

ε-CON (EDGELESS controller)

  * Exposes the `ControllerAPI` consisting of the `WorkflowInstanceAPI`
  * Binary: `edgeless_con_d`

### edgeless_bal

ε-BAL (EDGELESS balancer)

  * Manages resources, e.g., HTTP ingress/egress, and exposes a `ResourceConfigurationAPI` to
    configure it.
  * Binary: `edgeless_bal_d`


## Tools

The following tools allow users to test and interact with the EDGELESS project.

### edgeless_cli

CLI to interact with the e-Controller
    * Binary: `edgeless_cli`

### edgeless_inabox

A standalone EDGELESS environment with preconfigured instances of all required services inside a single binary.
This is the easiest way to spawn up a development instance of edgeless

## Libraries

In addition to the core components and tools, the repository contains the
following libraries:

* `edgeless_api`: Crate defining all inter-service APIs
    * Contains GRPC implementations of those inter-service APIs

* `edgeless_function`: Crate defining the interfaces for the tier1 guests
    * Tier1 Guests: Rust Functions compiled to WASM
    * Interface to the functions relies on the WASM component model.

* `edgeless_dataplane`: Crate defining the Edgeless data plane.
    * Provides the primary communication chains
        * Used by communicating entities to send and receive events
    * Provides a local communication provider
    * Provides a remote communication provider based on the `InvocationAPI`

* `edgeless_http`: Crate containing HTTP-related types.
    * Specifies the interface between the Ingress and the functions consuming
      HTTP Events.
