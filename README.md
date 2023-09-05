# Edgeless MVP Implementation

This repository contains a research prototype of the Edgeless platform.

*There currently are no guarantees on Stability and API-Stability!*

## How to build:

The Implementation relies on Rust and the ProtoBuf compiler.

The easiest way to get started is the devcontainer shipped as part of this
repository. See [Extra](#extra) for tips on how to properly set up the dev
container.

Build Steps:

Build edgeless core components and tools:

```
cargo build
```

### NixOS

If using Nix / on NixOS then there is a simple [`flake.nix`](./flake.nix) that is invoked via the `direnv` [`.envrc`](./.envrc) to autoinstall Nix package dependencies and give you a bulid shell once you `direnv allow` in this directory.

To build the function examples under `./examples` you will need to add the WASM toolchain via `rustup`:

```shell
rustup target add wasm32-unknown-unknown
```

## How to run:

It is recommended that you enable at least info-level log directives with:

```
export RUST_LOG=info
```

To get the basic system running, first create the default configuration files
(they have fixed hardcoded values):

```
target/debug/edgeless_inabox -t 
target/debug/edgeless_cli -t cli.toml
```

which will create:

- `balancer.toml`
- `controller.toml`
- `node.toml`
- `orchestrator.toml`
- `cli.toml`

Then you can run the **EDGELESS-in-a-box**, which is a convenience binary that
runs every necessary components as one, using the generated configuration files:

```
target/debug/edgeless_inabox
```

Congratulations, now a full EDGELESS system in running for you, although it is
not doing much.

To see examples of simple workflows composed of simple functions, look into the
`examples/` directory. There you will find a `README` file with a short
description of each example. Then, each example directory will contain a
detailed description of the example application.


## How to create new functions/workflows:

Please refer to
[documentation/rust_functions.md](documentation/rust_functions.md) and
[documentation/workflows.md](documentation/workflows.md).


## Repository Layout

Each of the main services is implemented as its own crate (library + (optional)
binary wrapper). Refer to the `README` file of a particular component to find
out more. To learn more about the conventions of Rust's module system, visit
[link](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html).

### Core Components
These represent the core components of the EDGELESS project, as proposed in the
design phase.

#### edgeless_node
Worker Node including the agent and the function runtime.
  * Exposes the `AgentAPI` consisting of the `FunctionInstanceAPI`
  * Exposes the `InvocationAPI` (data plane)
  * Binary: `edgeless_node_d`

#### edgeless_orc
e-Orchestrator. 
    * Exposes the `OrchestratorAPI` consisting of the `FunctionInstanceAPI`
    * Binary: `edgeless_orc_d`

#### edgeless_con
e-Controller
    * Exposes the `ControllerAPI` consisting of the `WorkflowInstanceAPI`
    * Binary: `edgeless_con_d`

#### edgeless_bal
e-Balancer
    * Contains the HTTP-Ingress and exposes a `ResourceConfigurationAPI` to
      configure it.
    * Binary: `edgeless_bal_d`



### Tools
The following tools allow users to test and interact with the EDGELESS project.

#### edgeless_cli
CLI to interact with the e-Controller
    * Binary: `edgeless_cli`

#### edgeless_inabox
A standalone EDGELESS environment with preconfigured
  instances of all required services inside a single binary. This is the easiest
  way to spawn up a development instance of edgeless


### Libraries
In addition to the core components and tools, the repository contains the
following libraries:

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
    * Specifies the interface between the Ingress and the functions consuming
      HTTP Events.


## Extra

1. It makes sense to clone the repository directly into a devcontainer to avoid
bind mounts and possibly make builds faster. To do this install VSCode, and
select: `DevContainers: Clone Repository in Named Container Volume`. It should
prompt you to a github page in your browser where you can authentificate. On an
M1 Max the achieved speedup was around x10 for `cargo build`.

2. There is a script to configure some plugins for zsh:
`scripts/enhance_zsh_dev_container.sh`, which is entirely optional. After
running it things like autocompletion and shell syntax highlighting are
available. Fell free to modify it to your liking!

3. If your build times are still horrible, try to allocate more CPUs and RAM to
   the Docker dev_container.

## Contributing

This section contains some rules you should adhere to when contributing to this
repository.

* Run the rust formatter before committing - `cargo fmt`. This ensures we
  minimize the noise coming from, e.g., whitespace changes.
* Try to limit the number of warnings (ideally, there should not be any
  warnings). A good way to do this is to run `cargo fix` before running the
  formatter.
    *  Suggested workflow: `cargo fix --allow-staged --allow-dirty && cargo fmt
       && git commit`
* When working on a new feature / issue, create a branch from the github issue
  and add your changes there. To merge the changes into the main, create a pull
  request and assign someone as a reviewer. The reviewer should then reject or
  accept the changes / leave some comments. After the changes are accepted by
  the reviewer, he should take care to merge them and remove the dangling
  feature branch.
* Do not introduce merge commits on the main branch. Merges to the main branch
  must be fast-forwarded. A good practice is also to squash the commits on the
  feature branch (can be done while merging on github).
* Add yourself to the list of contributors & adhere to the license.
    * Do not taint this repository with incompatible licenses!
    * Everything not MIT-licensed must be kept external to this repository.

## License

The Repository is licensed under the MIT License. Please refer to
[LICENSE-MIT.txt](LICENSE-MIT.txt) and [CONTRIBUTORS.txt](CONTRIBUTORS.txt). 
