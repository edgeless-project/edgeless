### Ping-pong example

The example creates a chain of two functions: ping and pong. The ping function
wakes up every 1 second and invokes the pong function, which merely terminates
after replying.

First, you have to locally build the WASM binaries:

```
target/debug/edgeless_cli function build examples/ping_pong/ping/function.json
target/debug/edgeless_cli function build examples/ping_pong/pong/function.json
```

which will generate the files:

- `examples/ping_pong/ping/pinger.wasm`
- `examples/ping_pong/pong/ponger.wasm`

Then, you can request the controller to start the workflow:

```
ID=$(target/debug/edgeless_cli workflow start examples/ping_pong/workflow.json)
```

Now `$ID` contains the workflow identifier assigned by the controller.

You will observe on the logs that the pinger workflow is, indeed, invoked every
1 second. Furthermore, a counter is increased at every new invocation. This
counter is the _state_ of the workflow, which is shared across multiple
instances of this workflow and persists after their termination.

For example, if you stop the worfklow:

```
target/debug/edgeless_cli workflow stop $ID
```

and you start again the workflow later, you will see the counter resuming from
the previous value (search for `{"count":NUM}` in the EDGELESS-in-a-box logs):

```
target/debug/edgeless_cli workflow start examples/ping_pong/workflow.json
```

You can always list the active workflows with:

```
target/debug/edgeless_cli workflow list
```