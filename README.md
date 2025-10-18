![](documentation/edgeless-logo-alpha-200.png)

# EDGELESS Reference Implementation

This repository contains a research prototype of the EDGELESS platform, which is
under active development within the project
[EDGELESS](https://edgeless-project.eu/) (2023-2025).

## Introduction

EDGELESS is a framework that enables
[serverless edge computing](documentation/serverless_edge_computing.md) and it
is intended especially for edge nodes with limited computational capabilities.

An EDGELESS cluster is managed by an ε-CON (controller) and consists of one
or more _orchestration domains_, each managed by an ε-ORC (orchestrator).

The ε-CON allows clients to request the creation of edge services (called
_workflows_), which consist of a collection of interconnected _functions_
and _resources_ and related annotations.
The management of the lifecycle of any such logical functions/resources is 
delegated by the ε-CON to one ε-ORC, which, in turn, manages the lifecycle
of physical function/resource instances on EDGELESS nodes.

Each EDGELESS node may offer multiple run-time environments (e.g., WebAssembly
or Docker) to run function instances and resource providers of different types
(e.g., file logs or Kafka) to create resource instances.

Orchestration in EDGELESS happens at two levels:

- _higher level orchestration_ is done by the ε-CON at cluster level
  and it maps logical functions/resources to orchestration domains;
- _lower level orchestration_ is done by the ε-ORC within its orchestration
  domain, and it maps every logical function/resource to physical instances
  on its nodes.

## How to build

See [building instructions](BUILDING.md).

## Quick start

It is recommended that you enable at least info-level log directives with:

```shell
export RUST_LOG=info
```

To get the basic system running, first create the default configuration files
(they have fixed hardcoded values):

```shell
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
runs every necessary component as one, using the generated configuration files:

```
target/debug/edgeless_inabox
```

Congratulations 🎉 now that you have a complete EDGELESS system you may check
our workflows/function examples, which are representative of the current
EDGELESS features:

- [Example workflows](examples/README.md)
- [Example functions](functions/README.md)

## Next steps

Basics:

- [Workflows, resources, and functions](documentation/basic_concepts.md)
- [ε-CON](documentation/controller.md)
- [ε-ORC](documentation/orchestrator.md)
- [EDGELESS node](documentation/node.md)
- [EDGELESS command-line clients](documentation/cli.md)
- [A step-by-step example](documentation/deploy_step_by_step.md)

Advanced topics:

- [Repository layout](documentation/repository_layout.md)
- [EDGELESS APIs](edgeless_api/README.md)
- [How to create a new function](documentation/rust_functions.md)
- [Local orchestration](documentation/local_orchestration.md)
- [Benchmarking EDGELESS](documentation/benchmark.md)
- [Docker container runtime](documentation/container-runtime.md)
- [A multi-domain example](documentation/example_multidomain.md)
- [Inter-domain workflows](documentation/interdomain_workflows.md)

## Known limitations

Currently there are several known limitations, including the following ones:

- No workflow-level annotations are supported.
- The payload of events is not encrypted.
- There currently are no guarantees on stability and API stability.

The full list of issues is tracked on
[GitHub](https://github.com/edgeless-project/edgeless/issues).

Stay tuned (star & watch
[the GitHub project](https://github.com/edgeless-project/edgeless)) to remain
up to date on future developments.

## Contributing

We love the open source community of developers ❤️ and we welcome contributions
to EDGELESS.

The [contributing guide](CONTRIBUTING_GUIDE.md) contains some rules you should
adhere to when contributing to this repository.

## License

The Repository is licensed under the MIT License. Please refer to
[LICENSE](LICENSE) and [CONTRIBUTORS.txt](CONTRIBUTORS.txt). 

## Funding

EDGELESS received funding from the [European Health and Digital Executive Agency
(HADEA)](https://hadea.ec.europa.eu/) program under Grant Agreement No
101092950.
