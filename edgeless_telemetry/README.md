# Edgeless Telemetry

This is an initial take on telemetry (to be extended / replaced as needed). Uses
OpenMetrics through this library: https://github.com/prometheus/client_rust.

Dockerfiles for running a prometheus instance and Grafana istance are provided
with a basic dashboard for a one-node Edgeless s=ystem. Currently used by the
edgeless_node to provide metrics such as:
- function_count - number of function instances that are present
- execution_times_count - number of times a function was executed
etc.
- execution_times_bucket - 

This has been tested on a local setup with Docker for Mac and edgeless_inabox
running locally.

## How to run Prometheus / Grafana as docker containers?
Make sure you have Docker Compose installed. Navigate to `components` and run:

```bash
docker-compose up --build -d
```

Then just open `localhost:3000` in your browser and open the `Edgeless default
dashboard` on the left.

## How to add new metric types?
TODO

## How to instrument your code?
Instrumentation is currently added to edgeless_node, check it out to learn more.

## Next steps:
- [x] Grafana dashboard for a cluster of one node
- [ ] Add function_class as a label for metrics
- [ ] expand Instructions on how to add metrics
- [x] default anonymous user