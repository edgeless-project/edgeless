# Edgeless node

- TODO: add links to readmes
- TODO: how much info here vs in the separate readmes? maybe no nested readmes?


- bin/ - contains the binary crate of this package

modules contained in this library
- agent/ - main logic of the node; manages the runners and resources; translates
  requests from the orchestrator into concrete on-node actions like e.g.
  starting of a function; exposes the AgentAPI, which has the
  FunctionInstanceAPI + InvocationAPI

- base_runtime/ - module containing the base trait that needs to be implemented
  by any other runner; contains guest_api (GuestAPIHost) which when imported
  allow runners to call e.g. dataplane functions (e.g. to call and get a result)

- gpu_info/ - helpers to extract the GPU state

- resources/ - implementation of the node resources, see readmes in specific
  resources for more info; readme in resources/ on how to create a new one

- runners/
    - container_runner - docker container runner
    - wasmtime_runner - default wasm runner; more performant of the two, but
      non-deterministic, supports WASI (webassembly system interface - file i/o,
      networking)
    - wasmi_runner - experimental runner that uses wasmi; has to be enabled in node
  config; theoretically better

- state_management/ - TODO

- node_subscriber.rs/ - TODO: move it somewhere where it would make sense
