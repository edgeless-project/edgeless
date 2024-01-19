// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use anyhow::anyhow;
use clap::Parser;
use edgeless_api::controller::ControllerAPI;
use edgeless_api::workflow_instance::{SpawnWorkflowResponse, WorkflowFunction, WorkflowId, WorkflowInstanceAPI};
use rand::prelude::*;
use rand_distr::Exp;
use rand_pcg::Pcg64;
use std::collections::BTreeMap;
use std::time;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    /// URL of the controller
    #[arg(short, long, default_value_t = String::from("http://127.0.0.1:7001"))]
    controller_url: String,
    /// Duration of the benchmarking experiment, in s
    #[arg(short, long, default_value_t = 30.0)]
    duration: f64,
    /// Average lifetime duration of a workflow, in s
    #[arg(short, long, default_value_t = 5.0)]
    lifetime: f64,
    /// Average inter-arrival between consecutive workflows, in s
    #[arg(short, long, default_value_t = 5.0)]
    interarrival: f64,
    /// Seed to initialize the pseudo-random number generators
    #[arg(short, long, default_value_t = 42)]
    seed: u64,
    /// Workflow type.
    #[arg(short, long, default_value_t = String::from("single;examples/noop/noop_function/function.json;examples/noop/noop_function/noop.wasm"))]
    wf_type: String,
}

enum Event {
    WfNew(),
    WfEnd(String),
}

static MEGA: u64 = 1000000;

fn to_seconds(us: u64) -> f64 {
    us as f64 / MEGA as f64
}

fn to_microseconds(s: f64) -> u64 {
    (s * MEGA as f64).round() as u64
}

enum WorkflowType {
    Single(String, String),
}

fn workflow_type(wf_type: &str) -> anyhow::Result<WorkflowType> {
    let tokens: Vec<&str> = wf_type.split(";").collect();
    if !tokens.is_empty() && tokens[0] == "single" && tokens.len() == 3 {
        return Ok(WorkflowType::Single(tokens[1].to_string(), tokens[2].to_string()));
    }
    Err(anyhow!("unknown workflow type: {}", wf_type))
}

struct ClientInterface {
    client: Box<dyn WorkflowInstanceAPI>,
    wf_type: WorkflowType,
}

impl ClientInterface {
    async fn new(controller_url: &str, wf_type: WorkflowType) -> Self {
        Self {
            client: edgeless_api::grpc_impl::controller::ControllerAPIClient::new(controller_url)
                .await
                .workflow_instance_api(),
            wf_type,
        }
    }

    async fn start_workflow(&mut self) -> anyhow::Result<String> {
        let mut functions: Vec<WorkflowFunction> = vec![];

        match &self.wf_type {
            WorkflowType::Single(path_json, path_wasm) => {
                let func_spec: edgeless_cli::workflow_spec::WorkflowSpecFunctionClass =
                    serde_json::from_str(&std::fs::read_to_string(path_json.clone()).unwrap()).unwrap();

                functions.push(WorkflowFunction {
                    name: "single".to_string(),
                    function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                        function_class_id: func_spec.id,
                        function_class_type: func_spec.function_type,
                        function_class_version: func_spec.version,
                        function_class_inlude_code: std::fs::read(path_wasm).unwrap(),
                        outputs: func_spec.outputs,
                    },
                    output_mapping: std::collections::HashMap::new(),
                    annotations: std::collections::HashMap::new(),
                });
            }
        }

        let res = self
            .client
            .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
                workflow_functions: functions,
                workflow_resources: vec![],
                annotations: std::collections::HashMap::new(),
            })
            .await;
        match res {
            Ok(response) => match &response {
                SpawnWorkflowResponse::ResponseError(err) => Err(anyhow!("{}", err)),
                SpawnWorkflowResponse::WorkflowInstance(val) => Ok(val.workflow_id.workflow_id.to_string()),
            },
            Err(err) => {
                panic!("error when stopping a workflow: {}", err);
            }
        }
    }

    async fn stop_workflow(&mut self, uuid: &str) -> anyhow::Result<()> {
        let res = self.client.stop(WorkflowId::from_string(uuid)).await;
        match res {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();

    let mut rng = Pcg64::seed_from_u64(args.seed);

    let lifetime_rv = Exp::new(1.0 / args.lifetime).unwrap();
    let interarrival_rv = Exp::new(1.0 / args.interarrival).unwrap();

    let mut client_interface = ClientInterface::new(&args.controller_url, workflow_type(&args.wf_type)?).await;

    // event queue, the first event is always a new workflow arriving at time 0
    let mut events = BTreeMap::new();
    events.insert(0 as u64, Event::WfNew()); // in us

    // main experiment loop
    let mut wf_started = 0;
    let mut wf_requested = 0;
    let mut now = 0;
    while now < to_microseconds(args.duration) {
        if let Some((event_time, event_type)) = events.pop_first() {
            // wait until the event
            assert!(event_time >= now);
            std::thread::sleep(time::Duration::from_micros(event_time - now));

            // handle the event
            now = event_time;
            match event_type {
                Event::WfNew() => {
                    wf_requested += 1;
                    match client_interface.start_workflow().await {
                        Ok(uuid) => {
                            wf_started += 1;
                            let lifetime = lifetime_rv.sample(&mut rng);
                            log::info!("{} new wf created '{}', will last {} s", to_seconds(now), &uuid, lifetime);
                            events.insert(now + to_microseconds(lifetime), Event::WfEnd(uuid));
                        }
                        Err(_) => {}
                    }
                    events.insert(now + to_microseconds(interarrival_rv.sample(&mut rng)), Event::WfNew());
                }
                Event::WfEnd(uuid) => {
                    log::info!("{} wf terminated  '{}'", to_seconds(now), &uuid);
                    match client_interface.stop_workflow(&uuid).await {
                        Ok(_) => {}
                        Err(err) => {
                            panic!("error when stopping a workflow: {}", err);
                        }
                    }
                }
            }
        }
    }

    // terminate all workflows that are still active
    for (_, event_type) in &events {
        if let Event::WfEnd(uuid) = event_type {
            match client_interface.stop_workflow(&uuid).await {
                Ok(_) => {}
                Err(err) => {
                    panic!("error when stopping a workflow: {}", err);
                }
            }
        }
    }

    // output metrics
    let blocking_probability = 1.0 - wf_started as f64 / wf_requested as f64;

    log::info!("workflow requested   = {}", wf_requested);
    log::info!("workflow started     = {}", wf_started);
    log::info!("blocking probability = {}", blocking_probability);

    Ok(())
}
