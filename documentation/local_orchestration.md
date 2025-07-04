- [Local orchestration in EDGELESS](#local-orchestration-in-edgeless)
  - [Delegated orchestration through a proxy](#delegated-orchestration-through-a-proxy)
  - [Node's telemetry](#nodes-telemetry)
  - [Step-by-step examples](#step-by-step-examples)
    - [Prerequisites](#prerequisites)
    - [Preparation steps](#preparation-steps)
    - [Example#1: telemetry and application metrics](#example1-telemetry-and-application-metrics)
    - [Example#2: delegated orchestration](#example2-delegated-orchestration)

# Local orchestration in EDGELESS

The ε-ORC implements a basic orchestration policy that:

1) honors the deployment requirements, and
2) ensures that one function instance is maintained in execution for all the
   accepted logical functions, if possible based on the deployment requirements.

If it is currently not possible to maintain in execution a function instance of
a given logical function, the ε-ORC will continue trying to create the
function or resource instance.

In all cases, the ε-ORC ensures that "patching", i.e., the interconnections
among function instances and resources for the exchange of events, is kept
up-to-date with the current components in execution.

Algorithms:

- If there are multiple resource providers that can host a resource,
  the ε-ORC selects one at random.
- If there are multiple nodes that can host a function instance, the ε-ORC
  uses one of the two basic strategies (which can be selected in the
  configuration file with `orchestration_strategy`):
  - `Random`: each node is assigned a weight equal to the product of the
  advertised number of CPUs, advertised number of cores per CPU, and
  advertised core frequency; then the node is selected using a weighted
  uniform random distribution;
  - `RoundRobin`: the ε-ORC keeps track of the last node used and
  assigns the next one (with wrap-around) among those eligible; note that
  this strategy does _not_ guarantee fairness if functions with different
  deployment requirements are requested.

The ε-ORC offers two optional mechanisms through a proxy:

1. Exposing the interval status and enabling delegated orchestration.
2. Collecting application metrics.

The following diagram illustrates these mechanisms, which are described separately below.

![](orchestrator-delegated-orc.png)

## Delegated orchestration through a proxy

This feature requires:

- an external in-memory database, e.g., Redis;
- the proxy feature enabled at the ε-ORC (see the
  [orchestrator's doc](orchestrator.md) for more details).

The proxy mirrors the internal data structures of the ε-ORC, so that an external
component, called **delegated orchestrator**  can make its decisions on which
function/resource instance should be executed on which node.

Such decisions are implemented by submitting _migration intents_ from the
delegated orchestrator to the ε-ORC through the proxy and they will be promptly
enforced, if possible.
If the migration is not feasible according to the deployment requirements,
the intent will be ignored by the ε-ORC.
For instance, if the latter receives a request to migrate a function instance
for which only nodes running in a TEE are allowed to a node that is not running
in a TEE, the ε-ORC will not enforce the intent.

The Redis proxy is enabled by means of the following section in `orchestrator.toml`: 

```ini
[proxy]
proxy_type = "Redis"
redis_url = "redis://127.0.0.1:6379"
```

To migrate the function with logical identifier `FID` to the node with
identifier `NODE` by operating manually through Redis, the delegated
orchestrator has two update two keys in the in-memory database:

1. Set the key `intent:migrate:FID` to `NODE`
2. Append the key `intent:migrate:FID` to the list `intents`

Multiple intents can be submitted at the same time: the ε-ORC will process them
in order from head to tail.

The command-line utility `proxy_cli` can be used as a convenient alternative:

```shell
proxy_cli intent migrate FID NODE
```

## Node's telemetry

EDGELESS nodes embed a telemetry system that collects some events related to
function lifecyle management, which is shown in the diagram below.

![](functions-state-diagram.png)

The telemetry sub-system also processes other types of events: function
instance exit (with termination status) and application-level log directives,
which can be added by the developers via `telemetry_log()` methods.

The processing of such telemetry events is configured in the `[telemetry]`
section of the node configuration file, for instance:

```ini
[telemetry]
metrics_url = "http://127.0.0.1:7003"
performance_samples = true
```

Where:

- `metrics_url`: URL of a web server that is exposed by the node with aggregated
  metrics using the [Prometheus](https://prometheus.io/) format. This is
  intended for throubleshooting purposes or to collect detailed data per node
  within a given orchestration domain by means of a process independent from
  the core EDGELESS ecosystem of tools; the web server can be disabled by
  specifying an empty string.
- `performance_samples`: if true, then sends the function execution/trasfer
  times and other custom log events via `telemetry_log` to the ε-ORC when
  refreshing the registration to the orchestration domain.

## Step-by-step examples

### Prerequisites

- A local copy of the edgeless repository is built in debug mode according to
  the [building instructions](../BUILDING.md).
- A Redis is reachable at 127.0.0.1:6379, see
  [online instructions](https://redis.io/docs/latest/operate/oss_and_stack/install/install-redis/).
- The current working directory is the root of the repository.
- [optional] `RUST_LOG=info ; export RUST_LOG`

### Preparation steps

In the following we will be running a minimal system with three nodes in a single
orchestration domain.
Create the default configuration files:

```bash
target/debug/edgeless_cli -t cli.toml
target/debug/edgeless_inabox -t -n 2
```

The latter will create the configuration files for the ε-CON, the ε-ORC, and two
nodes with WebAssembly run-times.

Modify `orchestrator.toml` so that the `[proxy]` section is the following:

```ini
[proxy]
proxy_type = "Redis"
redis_url = "redis://127.0.0.1:6379/"
proxy_gc_period_seconds = 0
```

Modify the configuration of node0 and node1 so that performance samples are
also shared with the ε-ORC (this is disabled by default when creating the
templates):

```shell
sed -i -e "s/performance_samples = false/performance_samples = true/" node[01].toml
```

Compile the WASM bytecode of the `vector_mul` function, which performs the
multiplication of an internal random matrix by the vector received as input, and
of the `message_generator` function, which produces periodically a message with
given given payload and a counter:

```bash
target/debug/edgeless_cli function build functions/vector_mul/function.json
target/debug/edgeless_cli function build functions/message_generator/function.json
```

### Example#1: telemetry and application metrics

Run the system:

```bash
target/debug/edgeless_inabox
```

Start a workflow consisting of three `vector_mul` functions in a chain:

```bash
target/debug/edgeless_cli workflow start examples/vector_mul/workflow-chain.json
```

The `proxy_cli` command-line utility can be used to show the orchestration
domain status, e.g., nodes' health:

```shell
target/debug/proxy_cli show node health
```

Example of output:

```
248ee9ad-0f27-44db-a0d8-702a5d5bcb7e -> memory free 850272 kb, used 22024192 kb, available 11853056 kb, process cpu usage 164%, memory 90832 kb, vmemory 412283456 kb, load avg 1 minute 150% 5 minutes 163% 15 minutes 181%, network tot rx 6316177408 bytes (75740707 pkts) 0 errs, tot tx 43359494 bytes (4881335296 pkts) 0 errs, disk available 994662584320 bytes, tot disk reads 125773818880 writes 84491526144, gpu_load_perc -1%, gpu_temp_cels -1.00°, active_power -1 mW
487ed9ee-2912-455e-9518-694a5731b05a -> memory free 850272 kb, used 22024192 kb, available 11853056 kb, process cpu usage 164%, memory 90832 kb, vmemory 412283456 kb, load avg 1 minute 150% 5 minutes 163% 15 minutes 181%, network tot rx 6316177408 bytes (75740707 pkts) 0 errs, tot tx 43359494 bytes (4881335296 pkts) 0 errs, disk available 994662584320 bytes, tot disk reads 125773818880 writes 84491526144, gpu_load_perc -1%, gpu_temp_cels -1.00°, active_power -1 mW
```

To show the current mapping of functions/resources to nodes:

```shell
target/debug/proxy_cli show node instances
```

Example of output:

```
248ee9ad-0f27-44db-a0d8-702a5d5bcb7e
[F] 9972cffd-4da8-4480-a5cb-b5ca1760f4b8
[F] a7a90f5a-f6c5-4f6c-9b04-99b0498dd1e1
487ed9ee-2912-455e-9518-694a5731b05a
[F] 44e2570b-bede-493e-bafe-e41f798bcc40
```

The three functions (`F`) in the chain are split between the two nodes with a
WebAssembly run-time.

The performance samples (collected by the nodes' telemetry) can dumped with:

```shell
target/debug/proxy_cli dump performance
```

The command will create several files:

- for each function there will be two files `UUID-function-execution_time.dat`
  and `UUID-function-transfer_time.dat`, with time series of the execution vs.
  transfer time of events
- one file ending with `tbegin.dat` and another with `tend.dat`: those files
  track the transaction vs. end, as emitted by custom `telemetry_log` calls
  in the `vector_mul` functions

### Example#2: delegated orchestration

Run the system:

```bash
target/debug/edgeless_inabox
```

Create a workflow consisting of a `message_generator` feeding a `file-log`
resource, which saves to a local file the content of the messages received,
optionally adding a timestamp, with the following command:

```bash
ID=$(target/debug/edgeless_cli workflow start examples/file_log/workflow.json)
```

In another shell you can see the content of `my-local-file.log` growing each
second:

```bash
tail -f my-local-file.log
```

Example of output:

```log
2024-12-12T11:56:30.023820+00:00 from node_id faaf87ba-9b46-4ff6-ac42-3fca12523128 function_id 597df9c4-db7f-4c5e-a716-bd8daeb7f480 [#0]: hello world
2024-12-12T11:56:31.061053+00:00 from node_id faaf87ba-9b46-4ff6-ac42-3fca12523128 function_id 597df9c4-db7f-4c5e-a716-bd8daeb7f480 [#1]: hello world
2024-12-12T11:56:32.285220+00:00 from node_id faaf87ba-9b46-4ff6-ac42-3fca12523128 function_id 597df9c4-db7f-4c5e-a716-bd8daeb7f480 [#2]: hello world
2024-12-12T11:56:33.287956+00:00 from node_id faaf87ba-9b46-4ff6-ac42-3fca12523128 function_id 597df9c4-db7f-4c5e-a716-bd8daeb7f480 [#3]: hello world
2024-12-12T11:56:34.460250+00:00 from node_id faaf87ba-9b46-4ff6-ac42-3fca12523128 function_id 597df9c4-db7f-4c5e-a716-bd8daeb7f480 [#4]: hello world
```

With the following command we can see the assignment of functions/resources
to nodes:

```shell
target/debug/proxy_cli show node instances
```

Example output:

```
4595df5d-21c9-43a5-8b69-006b85eced96
[F] ed0b963e-d6ff-4d62-84dc-7de9a2175913
[R] 16668152-2a27-4632-a482-336096c1be44
```

From the output we can see that the function `ed0b963e...` has been assigned
to node `4595df5d...`, like the resource.
Let us migrate the function to the other node:

```shell
target/debug/proxy_cli intent migrate \
  ed0b963e-d6ff-4d62-84dc-7de9a2175913 \
  faaf87ba-9b46-4ff6-ac42-3fca12523128
```

Running again:

```shell
target/debug/proxy_cli show node instances
```

We now see:

```
4595df5d-21c9-43a5-8b69-006b85eced96
[R] 16668152-2a27-4632-a482-336096c1be44
faaf87ba-9b46-4ff6-ac42-3fca12523128
[F] ed0b963e-d6ff-4d62-84dc-7de9a2175913
```

Note that the counter in `my-local-file.log` counter restarted from 0 upon
migrating, because it is kept in a function-local state
that is lost when the original function instance is terminated.