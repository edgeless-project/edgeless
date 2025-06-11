- [A multi-domain example](#a-multi-domain-example)
  - [Prerequisites](#prerequisites)
  - [Generation of the configuration files](#generation-of-the-configuration-files)
  - [Checking the cluster configuration](#checking-the-cluster-configuration)
  - [Creating workflows](#creating-workflows)
  - [Tearing down one orchestration domain](#tearing-down-one-orchestration-domain)


# A multi-domain example

In this guide we provide a step-by-step guide to deploying a minimal
EDGELESS system consisting of two orchestration domains, each with three
nodes, on a local machine.

_We assume good familiarity with the EDGELESS basics and command-line tools._

## Prerequisites

Check out a local copy of EDGELESS and compile the codebase in debug mode
following the [building guide](../BUILDING.md).

Build the `noop` function with:

```shell
target/debug/edgeless_cli function build ../../functions/noop/function.json
```

Create the configuration file for `edgeless_cli` with:

```shell
target/debug/edgeless_cli -t cli.toml
```

Enable information logging with:

```shell
export RUST_LOG=info
```

## Generation of the configuration files

The `edgeless_inabox` executable can be used to generate template
configuration files, also for multiple nodes.

Generate the configuration files for a full EDGELESS system, with three nodes,
in a directory called `primary`:

```shell
target/debug/edgeless_inabox --config-path primary -n 3 -t
```

Among the other information, the output will include
`Templates written, last port used: 7014`.

Create another set of configuration files in another directory called
`secondary`:

```shell
target/debug/edgeless_inabox --config-path secondary -n 3 -t --initial-port 7014
```

Remove from this directory the ε-CON's configuration file:

```shell
rm secondary/controller.toml
```

In one shell start the primary set of components:

```shell
cd primary ; ../target/debug/edgeless_inabox
```

And do the same for the secondary set of components, which is identical except
for the lack of an ε-CON:

```shell
cd secondary ; ../target/debug/edgeless_inabox
```

## Checking the cluster configuration

Congratulations, you should now have an EDGELESS system with one ε-CON and
two ε-ORCs, each overseeing 3 nodes.

You can double-check this by querying the ε-CON with the EDGLESS command-line
client:

```shell
target/debug/edgeless_cli domain list
```

will return:

```
domain domain-7014 (3 nodes)
domain domain-7000 (3 nodes)
```

Each domain can be further inspected with, e.g.:

```shell
target/debug/edgeless_cli domain inspect domain-7000
```

Example output (values will vary depending on the automatically inferred
nodes' capabilities):

```
3 nodes, 33 CPUs (33 cores) with 110592 MiB, labels [], num TEE 0, num TPM 0, runtimes [RUST_WASM], resources classes [dda,redis,http-egress,http-ingress,file-log] providers [redis-1,http-ingress-1,file-log-1,http-egress-1,dda-1], disk space 2845752 MiB, 0 GPUs with 0 MiB
```

## Creating workflows

Via the command-line client we can create new workflows, for instance using
the `noop` example, where each workflow consists of a single `noop` function,
which does nothing.

For example, let's create 10 workflows with a Bash loop:

```shell
for (( i = 0 ; i < 10 ; i++ )) ; do
  target/debug/edgeless_cli workflow start examples/noop/workflow.json
done
```

The list of active workflows can be retrieved with:

```shell
target/debug/edgeless_cli workflow list
```

Example output:

```
c46aa4e2-4731-4891-80fd-d1284f7a4639
00e8a73d-32cf-4eae-9e59-9bc9edd44d0d
e4a7f667-7a34-4da9-b0b4-66018683a542
bf4ba499-1622-4ea0-a77d-79975a08987c
ebc1c999-1d3e-4e1e-9b7d-9518ef97a2c4
953f8b45-d5ef-4e04-be41-503b90755a8c
3a726646-f6c8-43b2-86b1-ec29b3d021f8
5109ff8a-b766-4a6a-b090-6cef9ab58e58
a93b00f5-7b2d-46b8-8bdf-582271cff5eb
b1a38fd8-e83a-41e3-84ab-21e5057d4ae2
```

Each workflow can be inspected with the command-line client.
For example:

```shell
target/debug/edgeless_cli workflow inspect c46aa4e2-4731-4891-80fd-d1284f7a4639
```

Example output:

```
* function noop
run-time RUST_WASM class noop version 0.1
F_ANN init-payload -> nothing interesting
* mapping
MAP noop -> domain-7000 [logical ID 74837101-c684-4f2c-98e8-85f3a0354a69]
```

To check the mapping of all the functions in all the workflows we can use
some simple Bash scripting:

```shell
(for WF_ID in $(target/debug/edgeless_cli workflow list) ; do
    target/debug/edgeless_cli workflow inspect $WF_ID
done ) | grep ^MAP
```

Example output:

```
MAP noop -> domain-7000 [logical ID 74837101-c684-4f2c-98e8-85f3a0354a69]
MAP noop -> domain-7014 [logical ID 1880c3f6-79e3-4a2b-8d58-f40003419ba6]
MAP noop -> domain-7000 [logical ID 46e0360c-4f30-43eb-84e6-8d91901ebcba]
MAP noop -> domain-7014 [logical ID 718784b8-f0cb-4dd7-ac94-d5a09d2edaea]
MAP noop -> domain-7000 [logical ID 4c99fc42-e6c3-4192-8737-a121027743b6]
MAP noop -> domain-7014 [logical ID 804806dd-75b0-47be-8af5-0fea3201b4b5]
MAP noop -> domain-7014 [logical ID e333b196-5fd8-4d72-9a3c-394526ef3011]
MAP noop -> domain-7014 [logical ID 7425ffca-21bc-4602-ac78-d07838e9e59e]
MAP noop -> domain-7014 [logical ID 5dba5c7d-2c7f-46fd-be57-3e74b5482ed4]
MAP noop -> domain-7000 [logical ID bcfe0ce2-6f3a-4de6-849c-23ec7d4889c6]
```

You can migrate workflows from one domain to another, e.g.:

```shell
target/debug/edgeless_cli workflow migrate c46aa4e2-4731-4891-80fd-d1284f7a4639 domain-7014
```

## Tearing down one orchestration domain

Let us know stop all the `edgeless_inabox` in the `secondary` directory
with Ctrl+C.

The ε-CON will migrate all the functions previously assigned to the secondary
domain, i.e., `domain-7014`, to the primary one, i.e., `domain-7000`.

This can be verified by repeating the simple Bash script above, which this
time will produce as output:

```
MAP noop -> domain-7000 [logical ID 74837101-c684-4f2c-98e8-85f3a0354a69]
MAP noop -> domain-7000 [logical ID dcc7242b-3331-42bd-8758-2c6ae3f8610c]
MAP noop -> domain-7000 [logical ID 46e0360c-4f30-43eb-84e6-8d91901ebcba]
MAP noop -> domain-7000 [logical ID bb5871c8-0997-4546-a605-c1df56f32b14]
MAP noop -> domain-7000 [logical ID 4c99fc42-e6c3-4192-8737-a121027743b6]
MAP noop -> domain-7000 [logical ID 7280730c-8910-4467-9270-acf84c593828]
MAP noop -> domain-7000 [logical ID af44edc6-2dc5-47d6-afb8-a24280fbc9a1]
MAP noop -> domain-7000 [logical ID fd6da60e-9a82-49d3-ba47-2f48f924de7e]
MAP noop -> domain-7000 [logical ID 2bd6cbea-73b2-40c6-bc2c-8ea3bc55f5bc]
MAP noop -> domain-7000 [logical ID bcfe0ce2-6f3a-4de6-849c-23ec7d4889c6]
```

Restarting the secondary domain will make it possible for the ε-CON to assign
to it functions/resources associated with _new_ workflows.