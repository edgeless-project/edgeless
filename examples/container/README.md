### Container example

The example creates the following function chain:

```
                  output          output
sensor_simulator --------> delay --------> file-log
```

where:

- `sensor_simulator` periodically casts random number drawn from a
  configurable range: this will be deployed as a WASM function
- `delay` re-casts the event received towards its `output` channel after adding
  a delay of 1 second: this will be deployed as a Docker contain
- `file-log` is a resource that writes down the payload of events received
  to a local file

#### Function preparation

First, build the `sensor_simulator` WASM binary following the [instructions](../../functions/README.md). 

Then install Docker following the
[official instructions](https://docs.docker.com/get-docker/) and make sure
you can execute `docker` from the current user, e.g., the following command
should not give you a `permission denied` error:

```bash
docker ps
```

At this point, you can build a local container image called `edgeless_function`
with the following command, to be executed from the repo root:

```bash
docker build -t edgeless_function examples/container/
```

#### System preparation

The configuration of `edgeless_node` must include support for
Docker containers.

Let's assume that you plan to use `edgeless_inabox`, which will create
in the sample application all the core EDGELESS components (ε-CON, ε-ORC, and
a `edgeless_node`).

With:

```bash
target/debug/edgeless_inabox -t
target/debug/edgeless_cli -t cli.toml
```

you will create all the configuration files.

By default the configuration file `node.toml` does not support Docker containers,
so you must edit the file by changing the following section:

```ini
[container_runtime]
enabled = false
guest_api_host_url = "http://127.0.0.1:7100"
```

as follows (assuming `10.1.1.1` is an address of the computer where the
example is running):

```ini
[container_runtime]
enabled = true
guest_api_host_url = "http://10.1.1.1:7100"
```

**Important:** that the `guest_api_host_url` must point to a URL that is reachable
from the containers because the URL will be announced to the application
running in there. It must not be `127.0.0.1` or `0.0.0.0`.

After modifying `node.toml` as described above, you can start the
EDGELESS-in-a-box:

```bash
RUST_LOG=info target/debug/edgeless_inabox
```

#### Execution

You can start the workflow with:

```bash
ID=$(target/debug/edgeless_cli workflow start examples/container/workflow-rust.json)
```

You can check new values being saved, with an associated timestamp, to the
file specified in `workflow-rust.json`:

```bash
$ tail -f ./target/debug/my-local-file.log

2024-04-20T14:15:31.260558528+00:00 1.6461515
2024-04-20T14:15:32.262526211+00:00 0.3963747
2024-04-20T14:15:33.264510554+00:00 -0.6804714
2024-04-20T14:15:34.266705661+00:00 5.540745
2024-04-20T14:15:35.268476329+00:00 -1.5426998
```

The Docker container in execution is:

```bash
docker ps -f "ancestor=edgeless_function:latest"
```

Stop the workflow (and the running container) with:

```bash
target/debug/edgeless_cli workflow stop $ID
```

#### Python container alternative

You can replicate the above example by using a container running a function
developed in Python.

Clone a follow the instructions in
[this repository](https://github.com/edgeless-project/runtime-python), then
the only different is the use of `workflow-python.json` instead of
`workflow-rust.json`.