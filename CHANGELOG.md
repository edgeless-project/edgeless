# Changelog

## [Unreleased]

New features:

- Add ε-CON support to multiple orchestration domains.
- Implement dynamic cluster formation, with domain capabilities announced by
  the ε-ORCs (periodically refreshed).
- Refactor orchestration domain formation: the keep-alive mechanism has been
  removed from the ε-ORC, substituted by nodes periodically refreshing their
  registration.
- Add json-spec workflow type to edgeless_benchmark, which allows to create
  workflows based on a JSON template.
- New WASM function dup: duplicates the cast payload over two output channels.
- Add new proxy method to retrieve the domain identifier.
- Add new telemetry metric: function transfer time, which measures the time
  between when an event is generated and when it is handled by the intended
  worker.
- Add collection of execution/transfer time metrics for resources.
- Implement persistence across restarts of active workflows in ε-CON.
- Add CLI commands: workflow stop/inspect all, and domain inspect all.
- Add telemetry event logs to the data transmitted by the nodes to the ε-ORC.
- Add active power measurement to nodes from Modbus/TCP server intended for
  Raritan PDUS.
- Automatically add a label hostname:HOSTNAME to the node's capabilities.
- Add a serverless resource provider, which allows a node to offer resource
  instances that connect to an OpenFaaS-compatible function via an URL
  specified in the node's configuration. The resource has two output channels:
  "out" for the result of the function and "err" for functon execution errors.
- Add ε-CON support to workflow migration to a target domain.

Improvements:

- Remove need for components to be started in a specific order. The ε-CON,
  ε-ORC, and nodes can be started independently and automatically reconnect
  if the connection with their peer is lost.
- Improve scalability of the ε-ORC by separating control vs. management
  operations.
- Change behavior of announced URL in configuration files: when empty, use the
  IP address of the first non-loopback interface found.
- Proxy: add methods to fetch the dependency graph and to know if some category
  of data have been updated since the last fetch.
- Proxy: transform the performance samples and nodes' health into sorted sets
  to keep a history of previous values in the database, with periodic garbage
  collection done by the ε-ORC. Period duration is configurable.
- ε-ORC: reject nodes with empty host in invocation/agent URLs.
- Redis resource provider: add the option to read values from a Redis server.
- Add a flag to the resource providers' section of the node's configuration
  to prepend automatically the hostname to the resource providers' name.

API changes:

- Add service DomainRegistration.
- Update service NodeRegistration.
- Move node health status and performance samples from NodeManagement to
  NodeManagement.
- WorkflowInstance: return the list of workflow identifiers in list(); add new
  method inspect() to retrieve the workflow details.
- Remove unused fields:
  - ResourceInstanceSpecification::output_mapping
  - SpawnFunctionRequest::instance_id.
- Updated the content of the Event message in FunctionInvocation to include
  a timestamp of when the event was created.
- Add active_power to node health status
- Add WorkflowInstance::Migrate method, with associated messages.

## [1.0.0] - 2024-11-12

Initial stable release.

Notable features:
- multi-run-time support in nodes (WASM and Docker containers);
- resources supported: dda, file-log, http-egress, http-ingress,
  kafka-egress, ollama, redis;
- local observability supported at the ε-ORC through node telemetry;
- delegated orchestration implemented at the ε-ORC via a Redis proxy;
- benchmarking suite;

Notable limitations:
- the dataplane is limited within a single orchestration domain and realized
  through a full-mesh interconnection between all the nodes;
- the ε-BAL is a mere skeleton with no logic;
- the ε-CON only supports a single orchestration domain and does not perform
  any kind of admission control;
- no workflow-level annotations are supported; 
- the payload of events is not encrypted;
- the configuration of the ε-CON is read from a file and cannot be modified
  (e.g., it is not possible to add an orchestration domain);
- there is no persistence of the soft states of the various components.
