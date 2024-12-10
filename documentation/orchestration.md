# EDGELESS orchestration

Table of content:

- [EDGELESS orchestration](#edgeless-orchestration)
  - [Higher level orchestration (ε-CON)](#higher-level-orchestration-ε-con)
  - [Lower level orchestration (ε-ORC)](#lower-level-orchestration-ε-orc)
    - [Delegated orchestration through a proxy](#delegated-orchestration-through-a-proxy)
    - [Node's telemetry](#nodes-telemetry)
    - [Collection of application metrics](#collection-of-application-metrics)
    - [Step-by-step example](#step-by-step-example)

Orchestration in EDGELESS happens at two levels:

- _higher level orchestration_ is done by the ε-CON at cluster level (remember
  that a cluster may include multiple non-overlapping orchestration domains)
  and it maps (logical) function to orchestration domains;
- _lower level orchestration_ is done by the ε-ORC within its orchestration
  domain, and it maps every (logical) function to one or multiple workers
  running on the orchestration domain nodes.

## Higher level orchestration (ε-CON)

Work in progress.

## Lower level orchestration (ε-ORC)

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

### Delegated orchestration through a proxy

This feature currently requires an external Redis in-memory database, which is used to:

- mirror the internal data structures of the ε-ORC: these are updated periodically by ε-ORC and read by the delegated orchestrator to take its decisions, and
- receive orchestration intents: once the delegated orchestrator has taken a decision it informs the ε-ORC by updating the in-memory database with its intents, which will be promptly enforced, if possible.

_The in-memory database is flushed by the ε-ORC when it starts._

The Redis proxy is enabled by means of the following section in `orchestrator.toml`: 

```ini
[proxy]
proxy_type = "Redis"
redis_url = "redis://127.0.0.1:6379"
```

The ε-ORC internal status is serialized to Redis by means of the following entries:

| Key                                      | Value                                                                                                                                                                                                                  | `struct`                               |
| ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| nodes:capabilities:UUID                  | JSON object representing the capabilities of the node with given UUID                                                                                                                                                  | `NodeCapabilities`                     |
| node:health:UUID                         | JSON object representing the health status of the node with given UUID                                                                                                                                                 | `NodeHealthStatus`                     |
| performance:function_execution_time:UUID | List of function execution times of the function with the given physical UUID, in fractional seconds, each associated with a timestamp with a millisecond resolution taken by the ε-ORC (format `exec_time,timestamp`) | `NodePerformanceSamples`               |
| provider:ID                              | JSON object representing the configuration of the resource provider with given ID                                                                                                                                      | `ResourceProvider`                     |
| instance:UUID                            | JSON object including the annotations of the function with given logical UUID and the currently active instances (each with node identifier and physical function identifier)                                          | `ActiveInstance`                       |
| dependency:UUID                          | JSON object representing the dependencies of the function with given logical UUID through a map of output channel names to logical function identifiers                                                                | `HashMap<Uuid, HashMap<String, Uuid>>` |
|                                          |

Currently, we only support one intent type, which allows the delegated orchestrator to migrate one function instance from its current node to another.
Note that this operation must be feasible according to the deployment requirements, otherwise it will be ignored by the ε-ORC.
For instance, if the latter receives a request to migrate a function instance for which only nodes running in a TEE are allowed to a node that is not running in a TEE, the ε-ORC will not enforce the intent.

To migrate the function with logical identifier `FID` to the node with identifier `NODE`, the delegated orchestrator has two update two keys in the in-memory database:

1. Set the key `intent:migrate:FID` to `NODE`
2. Append the key `intent:migrate:FID` to the list `intents`

Multiple intents can be submitted at the same time: the ε-ORC will process them in order from head to tail.

We provide a command-line interface, called `proxy_cli`, which can be used
as a convenient alternative to manipulating directly the Redis database,
as shown in the step-by-step example below.

### Node's telemetry

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
log_level = "info"
performance_samples = true
```

Where:

- `metrics_url`: URL of a web server that is exposed by the node with aggregated
  metrics using the [Prometheus](https://prometheus.io/) format. This is
  intended for throubleshooting purposes or to collect detailed data per node
  within a given orchestration domain by means of a process independent from
  the core EDGELESS ecosystem of tools; the web server can be disabled by
  specifying an empty string.
- `log_level`: defines the logging level of the events that are not captured
  by the Prometheus-like web server above, which are appended to the regular
  `edgeless_node` logs; logging can be disabled by specyfing an empty string.
- `performance_samples`: if true, then sends the function execution times
  to the ε-ORC as part of the response to keep-alive messages (see
  `performance:function_execution_time:UUID` in the table above).

### Collection of application metrics

This feature currently requires an external Redis in-memory database, which is
used to store the metrics, and it is enabled by adding one node to the
orchestration domain that exposes a `metrics-collector` resource provider via
the following section in `node.toml`: 

```ini
[resources.metrics_collector_provider]
collector_type = "Redis"
redis_url = "redis://localhost:6379"
provider = "metrics-collector-1"
```

Currently two types of metrics are supported: `workflow` and `function`.
For both types the developer is responsible for:

- associating samples with a unique numerical identifier;
- indicating the beginning and end of the process being measured.

This can be done through the following invocations:

| Event                                                        | Code                                                           |
| ------------------------------------------------------------ | -------------------------------------------------------------- |
| A function-related process uniquely identified by `id` began | `cast("metric", format!("function:begin:{}", id).as_bytes());` |
| A function-related process uniquely identified by `id` ended | `cast("metric", format!("function:end:{}", id).as_bytes());`   |
| A workflow-related process uniquely identified by `id` began | `cast("metric", format!("workflow:begin:{}", id).as_bytes());` |
| A workflow-related process uniquely identified by `id` ended | `cast("metric", format!("workflow:end:{}", id).as_bytes());`   |

In the workflow composition, the application developer is responsible for
mapping the output with name `"metric"` of the function to `metrics-collector`.
The configuration of the latter includes a field `wf_name` which allows
specifying an identifier of the workflow.

The content of the in-memory database is the following.

| Key                      | Value                                                                                                                                                                                                                                                                                                |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| function:UUID:average    | A smoothed average of input samples received for the function with logical identifier UUID                                                                                                                                                                                                           |
| function:UUID:samples    | A list of values _sample_,_timestamp_, where _sample_ is the time (in ms) between function:begin and function:end for the function with physical identifier UUID and _timestamp_ is the time when function:end was received in fractional seconds since the Unix epoch with milliseconds granularity |
| workflow:WF_NAME:average | A smoothed average of input samples received for the workflow with identifier WF_NAME                                                                                                                                                                                                                |
| workflow:WF_NAME:samples | Same as function:UUID:samples but for the workflow with identifier WF_NAME                                                                                                                                                                                                                           |

Note that the metrics-collector automatically adds the _physical_ identifier of function instances for function-related metrics.
Multiple physical identifiers can be associated with a logical function during its lifetime.
The current mapping logical and physical identifier(s) can be found in the proxy information (instance:UUID entries).

### Step-by-step example

Prerequisites:

- A local copy of the edgeless repository is built in debug mode according to
  the [building instructions](../BUILDING.md).
- A Redis is reachable at 127.0.0.1:6379, see
  [online instructions](https://redis.io/docs/latest/operate/oss_and_stack/install/install-redis/).
- The current working directory is the root of the repository.
- The command-line utility `redis-cli` is installed.
- [optional] `RUST_LOG=info ; export RUST_LOG`

In the following we will be running a minimal system with two nodes in a single orchestration domain.
The instructions follow.

Create the default configuration files:

```bash
target/debug/edgeless_inabox -t
target/debug/edgeless_cli -t cli.toml
```

Modify the `node.toml` file so that `node_id` is
`fda6ce79-46df-4f96-a0d2-456f720f606c` and so that the metrics collector is
enabled with the following section:

```ini
[resources.metrics_collector_provider]
collector_type = "Redis"
redis_url = "redis://localhost:6379"
provider = "metrics-collector-1"
```

Modify the `orchestrator.toml` file so that the `[proxy]` section is:

```ini
[proxy]
proxy_type = "Redis"
redis_url = "redis://127.0.0.1:6379"

[collector]
collector_type = "Redis"
redis_url = "redis://127.0.0.1:6379"
```

Create the configuration file `node-2.toml` for another node, with a Rust
run-time, no resources associated, and default node's capabilities:

```ini
[general]
node_id = "fda6ce79-46df-4f96-a0d2-456f720f606d"
agent_url = "http://127.0.0.1:7221"
agent_url_announced = ""
invocation_url = "http://127.0.0.1:7202"
invocation_url_announced = ""
orchestrator_url = "http://127.0.0.1:7011"

[telemetry]
metrics_url = ""
performance_samples = true

[wasm_runtime]
enabled = true
```

In one shell run:

```bash
target/debug/edgeless_inabox
```

In another shell run:

```bash
target/debug/edgeless_node_d -c node-2.toml
```

Compile the WASM bytecode of the `vector_mul` function, which performs the
multiplication of an internal random matrix by the vector received as input:

```bash
target/debug/edgeless_cli function build functions/vector_mul/function.json
```

Start a workflow consisting of three `vector_mul` functions in a chain:

```bash
target/debug/edgeless_cli workflow start examples/vector_mul/workflow-chain.json
```

The full status of the in-memory database, including a mirror of the ε-ORC
internal data structures of the application metrics sampled, can be dumped with
a script provided:

```bash
scripts/redis_dump.sh
```

Or, more conveniently, it is possible to selective query the Redis through the
`proxy_cli` command-line utility provided.
For example, to show the nodes' heath status:

```shell
target/debug/proxy_cli show node health
```

Example of output:

```
03337f46-1dbe-41a1-94a4-75c0abc4e8f5 -> global cpu usage 20%, load 2, memory free 82208 kb, used 20916064 kb, total 37748736 kb, available 14934992 kb, process cpu usage 49%, memory 458032 kb, vmemory 420661520 kb
fda6ce79-46df-4f96-a0d2-456f720f606c -> global cpu usage 20%, load 2, memory free 82208 kb, used 20916064 kb, total 37748736 kb, available 14934992 kb, process cpu usage 49%, memory 458032 kb, vmemory 420661520 kb
fda6ce79-46df-4f96-a0d2-456f720f606d -> global cpu usage 20%, load 2, memory free 82048 kb, used 20916192 kb, total 37748736 kb, available 14934688 kb, process cpu usage 51%, memory 462160 kb, vmemory 429048624 kb
```

Note that three nodes are shown, but only two can run function instances, i.e.,
the one in the `edgeless_inabox` and that launched separately with the
configuration in `node-2.toml`.
The third node shown is the one embedded in the ε-ORC to host the
metrics-collector resource provider, as illustrated above, and it does not
have a run-time to execute function instances.

To show the current mapping of functions/resources to nodes:

```shell
target/debug/proxy_cli show node instances
```

Example of output:

```
03337f46-1dbe-41a1-94a4-75c0abc4e8f5
[R] 26828a53-21eb-4894-a723-4c4eeb9b6574
fda6ce79-46df-4f96-a0d2-456f720f606c
[F] cb314223-1021-428d-9df5-b73c53e258a2
fda6ce79-46df-4f96-a0d2-456f720f606d
[F] ee30d9c9-54c8-40e5-bfeb-cbcba527df05
[F] 7725a7a8-9871-447d-9203-a5fd117fd6ba
```

As you can see, the first node (the one embedded in the ε-ORC) is only assigned
one instance of type `R`, i.e., resource, while the three functions (`F`) are
split between the two nodes with a WebAssembly run-time.

With regard to performance samples (collected by the nodes' telemetry), they can
dumped to files with:

```shell
target/debug/proxy_cli dump performance
```

The command will create one file for each function instance containing the
timeseries of the execution times, for example (first 5 entries only):

```
0.243379291,1725557090.92
0.245919917,1725557090.92
0.238143375,1725557090.92
0.237986959,1725557090.92
0.241142625,1725557092.9
```

Where the first column contains the execution time, in fractional seconds,
and the second one the timestamp of when the performance sample was received
by the ε-ORC in response to a keep-alive.

Finally, since the `vector_mul` function supports application-related metrics,
these are also saved in Redis.

For instance, the average latency of the workflow can be queried with the
`redis-cli` command-line utility:

```bash
redis-cli get workflow:vector_mul_wf_chain:average
```

Where `vector_mul_wf_chain` is the name assigned to the workflow in
`workflow-chain.json`.

Example of output:

```
"930.679962190782"
```

Instead, the last 5 samples, with timestamps, are given by:

```bash
redis-cli lrange workflow:vector_mul_wf_chain:samples 0 4
```

Example of output:

```
1) "849,1718287852.177"
2) "958,1718287851.316"
3) "911,1718287850.347"
4) "896,1718287849.425"
5) "843,1718287848.516"
```

This completes the example on the collection of application metrics.
We now move to the delegated orchestration.

Compile the WASM bytecode of the `message_generator` function, which produces
periodically a message with given given payload and a counter:

```bash
target/debug/edgeless_cli function build functions/message_generator/function.json
```

Create a workflow consisting of a `message_generator` feeding a `file-log`
resource, which saves to a local file the content of the messages received,
optionally adding a timestamp, with the following command:

```bash
target/debug/edgeless_cli workflow start examples/file_log/workflow.json
```

In another shell you can see the content of `my-local-file.log` growing each
second:

```bash
tail -f my-local-file.log
```

Example of output:

```
2024-09-05T18:04:21.175674+00:00 from node_id fda6ce79-46df-4f96-a0d2-456f720f606d function_id 1a5a0386-2115-4188-8e15-a8c8b8561770 [#0]: hello world
2024-09-05T18:04:22.179790+00:00 from node_id fda6ce79-46df-4f96-a0d2-456f720f606d function_id 1a5a0386-2115-4188-8e15-a8c8b8561770 [#1]: hello world
2024-09-05T18:04:23.185131+00:00 from node_id fda6ce79-46df-4f96-a0d2-456f720f606d function_id 1a5a0386-2115-4188-8e15-a8c8b8561770 [#2]: hello world
```

This also tells us that the function instance of `message_generator` has been
assigned to the node `fda6ce79-46df-4f96-a0d2-456f720f606d`.
If we want to migrate the function instance to the other node, which has the
same UUID except for the last digit (`c` instead of `d`) then we need to know
what is the logical UUID of the function.
This can be retrieved, for instance, with `proxy_cli`:

```bash
target/debug/proxy_cli show logical-to-physical
```

Example output (look at the first entry):

```
02ccfc3d-8c9f-4a41-81c8-d4557cdb0c99 -> 1a5a0386-2115-4188-8e15-a8c8b8561770
9fd2e89c-e6ca-457c-ac64-465ac6ddcce0 -> c2a0cdfc-1ebf-4766-9dfd-2473315e6cab
```

At this point we can migrate the function to the node whose identifier ends
with `c` using `proxy_cli`, again:

```bash
target/debug/proxy_cli intent migrate \
  02ccfc3d-8c9f-4a41-81c8-d4557cdb0c99 \
  fda6ce79-46df-4f96-a0d2-456f720f606c
```

This will add an intent to Redis, which will promptly instruct the ε-ORC to
perform the migration.
This is visible from the content of the `my-local-file.log` which now contains

```
2024-09-05T18:08:26.475367+00:00 from node_id fda6ce79-46df-4f96-a0d2-456f720f606d function_id 1a5a0386-2115-4188-8e15-a8c8b8561770 [#244]: hello world
2024-09-05T18:08:27.442057+00:00 from node_id fda6ce79-46df-4f96-a0d2-456f720f606c function_id 70cb6b96-f418-4746-921e-bba6bc3a9466 [#0]: hello world
2024-09-05T18:08:28.444849+00:00 from node_id fda6ce79-46df-4f96-a0d2-456f720f606c function_id 70cb6b96-f418-4746-921e-bba6bc3a9466 [#1]: hello world
```

Note that:

- the identifier of the node now ends with `c` (this can be verified with
  `target/debug/proxy_cli show functions`);
- the counter restarted from 0, because it is kept in a function-local state
  that is lost when the function instance is migrated.