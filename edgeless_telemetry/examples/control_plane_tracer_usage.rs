// Example usage of the control plane tracer for orchestrator metrics

use edgeless_telemetry::control_plane_tracer::ControlPlaneTracer;

fn main() {
    // create tracer that writes to stdout
    let tracer = ControlPlaneTracer::new(String::new()).unwrap();

    // or write to a file
    // let tracer = ControlPlaneTracer::new("orchestrator_traces.csv".to_string()).unwrap();

    // start a span for a workflow deployment
    let deployment_span = tracer.start_span("deploy_workflow");
    deployment_span.log("info", "starting workflow deployment");

    {
        // child span for resource allocation
        let allocation_span = deployment_span.child("resource_allocation");
        allocation_span.log("debug", "selecting nodes");
        allocation_span.log("info", "allocated 3 nodes");
    }

    {
        // child span for function instantiation
        let instantiation_span = deployment_span.child("function_instantiation");
        instantiation_span.log("debug", "creating function instances");
        instantiation_span.log("info", "instantiated 5 functions");
    }

    deployment_span.log("info", "workflow deployment completed");

    // global log without correlation
    tracer.log("warn", "system load high");
}
