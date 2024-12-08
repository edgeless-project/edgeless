# Changelog

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
