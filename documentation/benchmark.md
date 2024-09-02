# Benchmark tools

## edgeless_benchmark

`edgeless_benchmark` is a tool to help function developers and designers of
orchestration algorithms through the automated performance evaluation of a
population of workflows in controlled conditions.

The tool supports different workload models and workflow types.
The duration of the experiment is configurable via a command-line option,
like the seed used to generate pseudo-random numbers to enable repeatable
experiments.

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

In one shell start the EDGELESS in-a-box:

```bash
target/debug/edgeless_inabox
```

In another run the following benchmark, which lasts 30 seconds:

```bash
target/debug/edgeless_benchmark -w "vector-mul-chain;5;5;1000;2000;functions/vector_mul/vector_mul.wasm
```

At the end you will find a file `out.csv` that contains the dump of the
metrics collected during the benchmark.

During the execution you can query the status of the ε-ORC and the metrics
by looking at the content of Redis.
A utility script is provided that dumps the content (except individual latency
samples) by pretty-print JSON values:

```bash
scripts/redis_dump.sh
```