# Edgeless Telemetry

Telemetry support for EDGELESS nodes and control plane components.

## Components

### Data Plane Telemetry (Nodes)

Event-based metrics collection for function execution and resource usage. Both targets track the same events but export them differently:

**PerformanceTarget** - Collects raw samples from function execution:
- Stores execution times, transfer times, and log entries per function
- Periodically sent from nodes to orchestrator via node registration refresh
- Orchestrator writes to Redis and optionally to CSV file
- Used for post-processing and analysis of function-level metrics

**PrometheusTarget** - Aggregates metrics for real-time monitoring using e.g. dashboards and alerts:
- Exposes HTTP endpoint (`metrics_url`) for Prometheus scraping
- Tracks node-level and function-level metrics in histograms
- Includes: `function_count`, `execution_times`, `transfer_times`
- Used for live dashboards (Grafana) and alerting

Both targets receive the same telemetry events from function runtimes:
- Function lifecycle: instantiate, init, exit
- Function execution: invocation completed times, transfer times
- Function logs: correlated to function instances

Uses OpenMetrics through https://github.com/prometheus/client_rust.

### Control Plane Tracer

Lightweight span-based tracer for orchestrator and controller operations. Designed for control plane, not data plane.

Exports traces to CSV format for analyzing orchestrator decision-making and workflow lifecycle.

**API**:
- `ControlPlaneTracer::new(output_path)` - create tracer (stdout or file)
- `tracer.start_span(name)` - create root span with correlation ID
- `span.child(name)` - create child span with parent reference
- `span.log(level, message)` - log correlated to span
- Automatic span end on drop (RAII)

**CSV Output Format**:
```
timestamp_sec,timestamp_ns,event_type,correlation_id,parent_id,name,level,message
```

**Why not OpenTelemetry?**

We implement a minimal subset of OpenTelemetry's tracing functionality tailored for orchestrator 
observability as we need a simple, low-overhead solution without external dependencies.

For production orchestrator observability with distributed tracing and existing infrastructure integration, use OpenTelemetry instead.

## Running Prometheus / Grafana

> This dashboard has not been updated for the latest telemetry metrics. Use it as a reference only.

Navigate to `components` and run:

```bash
docker-compose up --build -d
```

Open `localhost:3000` and access the Edgeless dashboard.