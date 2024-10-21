# EDGELESS repository layout

Each of the primary services in EDGELESS is implemented as its own Crate: `library` + (optional) `binary wrapper`.
If existing, please refer to the `README.md` file of a particular component to find out more.

You may want to check the [conventions of Rust's module system](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html).



| Directory                                              | Description |
| ------------------------------------------------------ |------------ |
| [dda/](../dda/README.md)                               | Crate for the [Data Distribution Agent](https://github.com/coatyio/dda) *resource* |
| [deployment/](../deployment/README.md)                 | Directory containing docker and docker compose scripts    <!-- TODO: Rename directory to something like docker-runtime or docker-scripts --> |
| docs/                                                  | Directory with miscellaneous documentation about the EDGELESS project |
| edgeless_api/                                          | Library crate with all the inter-service APIs definitions. Contains the GRPC implementations of those APIs, both for services and messages. This directory must be imported by other projects wishing to interact with EDGELESS components through its interfaces. The following interfaces are currently implemented: `s01`, `s04`, `s06`, `s07` |
| edgeless_api_core/                                     | Work-in-progress crate on minimal functions for embedded devices using CoAP |
| [edgeless_bal/](../edgeless_bal/README.md)             | Crate with a reference implementation of the ε-BAL. Currently a mere skeleton, the concrete implementation will be done in the next project phase when inter-domain workflows are be supported |
| [edgeless_benchmark/](../edgeless_benchmark/README.md) | Crate to benchmark an EDGELESS system in a controlled and deterministic environment using artificial workloads |
| edgeless_cli/                                          | Crate to build the EDGELESS command-line interface. Currently used to locally build function instances and to interact with the ε-CON via the `s04` interface to create/terminate/list workflows |
| edgeless_con/                                          | Crate to build the reference implementation of the ε-CON. Currently only supporting a single orchestration domain and ignoring workflow annotations |
| edgeless_container_function/                           | Crate to build the skeleton of a function to be deployed in a container |
| [edgeless_dataplane/](../edgeless_dataplane/README.md) | Library crate defining the EDGELESS intra-domain dataplane, which is realised through the full-mesh interconnection of gRPC services implementing the `s01` API |
| edgeless_embedded/                                     | Work-in-progress crate with the implementation of special features for embedded devices |
| edgeless_embedded_emu/                                 | Crate with an embedded device emulator |
| edgeless_embedded_esp32/                               | Crate for support of some ESP32 microcontrollers |
| edgeless_function/                                     | Libary crate with the definitions of the interfaces for the tier1 guests (Rust Functions compiled to WASM). The interface for the functions relies on the WASM component model. Contains the WebAssembly Rust bindings and function programming model |
| edgeless_http/                                         | Library crate containing utility structures and methods for HTTP bindings, and HTTP-related types. Specifies the interface between the Ingress and the functions consuming HTTP Events. |
| edgeless_inabox/                                       | Crate to build a minimal, yet complete, EDGELESS system consisting of an ε-CON, ε-ORC, ε-BAL and edgeless node within a single binary. Intended to be used for development/validation purposes |
| [edgeless_node/](../edgeless_node/README.md)           | Crate to build an EDGELESS node with WebAssembly and [Container](container-runtime.md) run-times |
| [edgeless_orc/](../edgeless_orc/README.md)             | Crate to build the reference implementation of the ε-ORC, supporting deployment annotations and implementing two simple function instance allocation strategies: random and round-robin. Upscaling is not supported: all the functions are deployed as single instances |
| edgeless_systemtests/                                  | Crate with tests of EDGELESS components deployed in a system fashion, e.g. interacting through gRPC interfaces |
| [edgeless_telemetr/y](../edgeless_telemetry/)          | Work-in-progress crate for a component that provides telemetry data regarding the EDGELESS operation, also supporting Prometheus agents |
| [examples/](../examples/README.md)                     | Directory with several examples showcasing the key features of the EDGELESS reference implementation |
| [functions/](../functions/README.md)                   | Library of _example functions_ shipping with the EDGELESS platform used by the aforementioned examples |
| model/                                                 | Work-in-progress OCaml model of the EDGELESS system |
| [schemas/](../schemas/README.md)                       | TODO |
| scripts/                                               | Directory containing various project-related scripts |

## Directory descriptions

### Core Components

These represent the core components of the EDGELESS project, as proposed in the
design phase.

#### edgeless_node

Worker node including the agent and the function runtime.
  * Exposes the `AgentAPI` consisting of the `FunctionInstanceAPI`
  * Exposes the `InvocationAPI` (data plane)
  * Binary: `edgeless_node_d`

#### edgeless_orc

ε-ORC (EDGELESS orchestrator)

  * Exposes the `OrchestratorAPI` consisting of the `FunctionInstanceAPI`
  * Binary: `edgeless_orc_d`

#### edgeless_con

ε-CON (EDGELESS controller)

  * Exposes the `ControllerAPI` consisting of the `WorkflowInstanceAPI`
  * Binary: `edgeless_con_d`

#### edgeless_bal

ε-BAL (EDGELESS balancer)

  * Manages resources, e.g., HTTP ingress/egress, and exposes a `ResourceConfigurationAPI` to
    configure it.
  * Binary: `edgeless_bal_d`


### Tools

The following tools allow users to test and interact with the EDGELESS project.

#### edgeless_cli

CLI to interact with the e-Controller
    * Binary: `edgeless_cli`

#### edgeless_inabox

A standalone EDGELESS environment with preconfigured instances of all required services inside a single binary.
This is the easiest way to spawn up a development instance of edgeless

### Libraries

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
