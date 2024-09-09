# Benchmark tools

## edgeless_benchmark

`edgeless_benchmark` is a tool to help function developers and designers of
orchestration algorithms through the automated performance evaluation of a
population of workflows in controlled conditions.

The tool supports different arrival models and workflow types.

Arrival models (option `--arrival-model`):

| Arrival model | Description                                                                                               |
| ------------- | --------------------------------------------------------------------------------------------------------- |
| poisson       | Inter-arrival between consecutive workflows and durations are exponentially distributed.                  |
| incremental   | One new workflow arrive every new inter-arrival time.                                                     |
| incr-and-keep | Add workflows incrementally until the warm up period finishes, then keep until the end of the experiment. |
| single        | Add a single workflow.                                                                                    |

Workflow types (option `--wf_type`):

| Workflow type    | Description                                                                                                                                                                                                                               | Application metrics |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| single           | A single function.                                                                                                                                                                                                                        | None                |
| matrix-mul-chain | A chain of functions, each performing the multiplication of two matrices of 32-bit floating point random numbers at each invocation.                                                                                                      | workflow,function   |
| vector-mul-chain | A chain of functions, each performing the multiplication of an internal random matrix of 32-bit floating point numbers by the input vector received from the caller.                                                                      | workflow,function   |
| map-reduce       | A workflow consisting of a random number of stages, where each stage is composed of a random number of processing blocks. Before going to the next stage, the output from all the processing blocks in the stage before must be received. | workflow            |

The duration of the experiment is configurable via a command-line option,
like the seed used to generate pseudo-random numbers to enable repeatable
experiments and the duration of a warm-up period.

## Dataset creation

The command `edgeless_benchmark` and the ε-ORC both support the option to save
run-time events during the execution for the purpose of creating a dataset from
an execution of the benchmark.
It is also possible to specify 
additional_fields

For `edgeleless_benchmark` this option is enabled by specifying a non-empty
value for option `--dataset_path`, which defines the path of where to save
the dataset files.
The dataset files are encoded in a comma-separated values (CSV) format, with
the first row in each file containing the column names.
Each entry is pre-prended with additional fields, which can be specified with
the `--additional_fields`, corresponding to the additional header
`--additional_header`.
The output files are overwritten unless the `--append` option is provided.

For the ε-ORC, a Redis proxy must be enabled, and an additional optional
section `[proxy.dataset_settings]` must be added, whose fields have the same
meaning as the corresponding `edgeless_benchmark` command-line options above
(see step-by-step example below).

The dataset files produced are the following:

| Filename                   | Format                                         | Produced by          |
| -------------------------- | ---------------------------------------------- | -------------------- |
| health_status.csv          | timestamp,node_id,node_health_status           | ε-ORC                |
| capabilities.csv           | timestamp,node_id,node_capabilities            | ε-ORC                |
| mapping_to_instance_id.csv | timestamp,logical_id,node_id1,physical_id1,... | ε-ORC                |
| performance_samples.csv    | metric,identifier,value,timestamp              | ε-ORC                |
| application_metrics.csv    | entity,identifier,value,timestamp              | `edgeless_benchmark` |

Notes:

- The timestamp format is always A.B, where A is the Unix epoch in seconds and
B the fractional part in nanoseconds.
- All the identifiers (node_id, logical_id, and physical_id) are UUID.
- The field entity in the application metrics can be `f` (function) or `w`
  (workflow).
- Check the difference between application metrics and performance samples
  in the [orchestration documentation](./orchestration.md).


### Step by step example

We assume that the repository has been downloaded and compiled in debug mode
(see [building instructions](../BUILDING.md)) and that a local instance of
Redis is running (see
[online instructions](https://redis.io/docs/latest/operate/oss_and_stack/install/install-redis/)).

First, build the `vector_mul.wasm` bytecode:

```bash
target/debug/edgeless_cli function build functions/vector_mul/function.json
```

Then, create the configuration files:

```bash
target/debug/edgeless_inabox -t
sed -i \
    -e "s/proxy_type = \"None\"/proxy_type = \"Redis\"/" \
    -e "s/redis_url = \"\"/redis_url = \"redis:\/\/127.0.0.1:6379\"/" \
    orchestrator.toml
```

Add the following section to `orchestrator.toml`:

```ini
[proxy.dataset_settings]
dataset_path = "dataset/myexp-"
append = true
additional_fields = "a,b"
additional_header = "h_a,h_b"
```

And create the directory where the dataset files will be created:

```shell
mkdir dataset
```

In one shell start the EDGELESS in-a-box:

```bash
target/debug/edgeless_inabox
```

In another run the following benchmark, which lasts 30 seconds:

```shell
target/debug/edgeless_benchmark \
    -w "vector-mul-chain;5;5;1000;2000;functions/vector_mul/vector_mul.wasm" \
    --dataset-path "dataset/myexp-" \
    --additional-fields "a,b" \
    --additional-header "h_a,h_b" \
    --append
```

The `dataset` directory now contains all the files in the table below,
starting with the prefix `myexp-`.

An example of a post-processing script is [included](examples-app-metrics.py):

```shell
% DATASET=dataset/myexp-application_metrics.csv python documentation/examples-app-metrics.py
the average latency of wf6 was 33.23 ms
the average latency of wf4 was 69.67 ms
the average latency of wf0 was 19.58 ms
the average latency of wf1 was 50.98 ms
the average latency of wf2 was 54.84 ms
the average latency of wf5 was 72.22 ms
```
