TODO: explain what outer is

- GuestAPIFunction - gets implemented by the concrete virtualization-technology
  function instance; acts as a gRPC server to the edgeless node's client, which
  allows it to manage the instance and interact with it; boot / init / cast /
  call / stop
- GuestAPIHost - implemented by the edgeless node; function instance connects to
  it as a client to perform actions like sending events or logging telemetry
  data

```
+----------------------------+                           +-----------------------------+
|     EDGELESS Node          |                           |     Function Instance       |
| (implements GuestAPIHost)  |                           |(implements GuestAPIFunction)|
+----------------------------+                           +-----------------------------+
           ^                                                             |
           |                                                             |
           |                  Init, Boot, Cast, Call, Stop               |
           |  -------------------------------------------------------->  |
           |                                                             |
           |                                                             v
           |                TelemetryLog, Cast, Call, Sync,              |
           |  <--------------------------------------------------------  |
           |                DelayedCast, CastRaw, CallRaw                |
           |                                                             |
           |                                                             |
+----------------------------+                           +-----------------------------+
|     gRPC Server            |                           |     gRPC Server             |
|  (GuestAPIHost interface)  |                           | (GuestAPIFunction interface)|
+----------------------------+                           +-----------------------------+
```

Legend:
- Function instance starts and runs a gRPC server for GuestAPIFunction.
- EDGELESS node connects to it to manage lifecycle and messaging.
- Function instance connects back to the node’s GuestAPIHost to trigger cross-function operations or telemetry.
