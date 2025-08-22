# Edgeless Dataplane

Implements the dataplane used between Components (functions,resources,balancers).

Currently, the dataplane supports sending basic unicast events to another component (`cast`),
as well as a higher level-mechanism for sending events that expect a return event (`call`).
Future versions of the system may add additional APIs/mechanisms, e.g., data-centric communication or anycast/multicast communication.

Each component has a `DataplaneHandle` that is used for both the events leaving the component (outgoing) and received by the component (incomming).
The `DataplaneHandle` is registered to the `InstanceId` of a component.

A handle for a component can be retrieved from the `DataplaneProvider`, of which there should be a single instance on each node.

A

Normal incomming events are received by blocking on `receive_next` provided by the `DataplaneHandle`.
Replies to `call` events are directly returned by the handle's `call` method.

There are currently two link(providers): The `node_local` link-type for on-node communication and the `remote_node` link-type allowing to communicate with a single node using the `InvocationAPI`.
