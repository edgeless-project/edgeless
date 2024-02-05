// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use anyhow::anyhow;
use clap::Parser;
use core::cmp::Ordering;
use edgeless_api::controller::ControllerAPI;
use edgeless_api::workflow_instance::{SpawnWorkflowResponse, WorkflowFunction, WorkflowId, WorkflowInstanceAPI};
use rand::prelude::*;
use rand::SeedableRng;
use rand_distr::Exp;
use rand_pcg::Pcg64;
use std::collections::BinaryHeap;
use std::time;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    /// URL of the controller
    #[arg(short, long, default_value_t = String::from("http://127.0.0.1:7001"))]
    controller_url: String,
    /// URL of the orchestrator
    #[arg(short, long, default_value_t = String::from("http://127.0.0.1:7011"))]
    orchestrator_url: String,
    /// Address to use to bind servers
    #[arg(short, long, default_value_t = String::from("127.0.0.1"))]
    bind_address: String,
    /// Arrival model, one of {poisson, incremental}
    #[arg(long, default_value_t = String::from("poisson"))]
    arrival_model: String,
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

#[derive(PartialEq, Eq)]
enum Event {
    /// 0: Event time.
    WfNew(u64),
    /// 0: Event time.
    /// 1: UUID of the workflow.
    WfEnd(u64, String),
    /// 0: Event time.
    WfExperimentEnd(u64),
}

impl Event {
    fn time(&self) -> u64 {
        match self {
            Self::WfNew(t) => *t,
            Self::WfEnd(t, _) => *t,
            Self::WfExperimentEnd(t) => *t,
        }
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.time().partial_cmp(&self.time())
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

enum ArrivalModel {
    /// Inter-arrival between consecutive workflows and durations are exponentially distributed.
    Poisson,
    /// One new workflow arrive every new inter-arrival time.
    Incremental,
}

static MEGA: u64 = 1000000;

fn to_seconds(us: u64) -> f64 {
    us as f64 / MEGA as f64
}

fn to_microseconds(s: f64) -> u64 {
    (s * MEGA as f64).round() as u64
}

enum WorkflowType {
    None,
    // 0: function.json path
    // 1: function.wasm path
    Single(String, String),
    // 0: min chain length
    // 1: max chain length
    // 2: min matrix size
    // 3: max matrix size
    // 4: interval between consecutive transactions, in ms
    //    if 0 then make the workflow circular, i.e., the last
    //    function calls the first one to trigger a new
    //    transaction
    // 5: matrix_mul.wasm path
    // 6: Redis URL
    MatrixMulChain(u32, u32, u32, u32, u32, String, String),
}

fn workflow_type(wf_type: &str) -> anyhow::Result<WorkflowType> {
    let tokens: Vec<&str> = wf_type.split(';').collect();
    if !tokens.is_empty() && tokens[0] == "none" {
        return Ok(WorkflowType::None);
    } else if !tokens.is_empty() && tokens[0] == "single" && tokens.len() == 3 {
        return Ok(WorkflowType::Single(tokens[1].to_string(), tokens[2].to_string()));
    } else if !tokens.is_empty() && tokens[0] == "matrix-mul-chain" && tokens.len() == 8 {
        return Ok(WorkflowType::MatrixMulChain(
            tokens[1].parse::<u32>().unwrap_or_default(),
            tokens[2].parse::<u32>().unwrap_or_default(),
            tokens[3].parse::<u32>().unwrap_or_default(),
            tokens[4].parse::<u32>().unwrap_or_default(),
            tokens[5].parse::<u32>().unwrap_or_default(),
            tokens[6].to_string(),
            tokens[7].to_string(),
        ));
    }
    Err(anyhow!("unknown workflow type: {}", wf_type))
}

struct ClientInterface {
    client: Box<dyn WorkflowInstanceAPI>,
    wf_type: WorkflowType,
    rng: rand::rngs::StdRng,
    wf_id: u32,
}

impl ClientInterface {
    async fn new(controller_url: &str, wf_type: WorkflowType) -> Self {
        Self {
            client: edgeless_api::grpc_impl::controller::ControllerAPIClient::new(controller_url)
                .await
                .workflow_instance_api(),
            wf_type,
            rng: rand::rngs::StdRng::from_entropy(),
            wf_id: 0,
        }
    }

    async fn start_workflow(&mut self) -> anyhow::Result<String> {
        let mut functions = vec![];
        let mut resources: Vec<edgeless_api::workflow_instance::WorkflowResource> = vec![];

        match &self.wf_type {
            WorkflowType::None => {}
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
            WorkflowType::MatrixMulChain(min_chain_size, max_chain_size, min_matrix_size, max_matrix_size, inter_arrival, path_wasm, redis_url) => {
                let chain_size: u32 = self.rng.gen_range(*min_chain_size..=*max_chain_size);

                let mut matrix_sizes = vec![];

                for i in 0..chain_size {
                    let mut outputs = vec!["metrics".to_string()];
                    for k in 0..20 {
                        outputs.push(format!("out-{}", k).to_string());
                    }
                    let mut output_mapping = std::collections::HashMap::from([("metric".to_string(), "metrics-collector".to_string())]);
                    if i != (chain_size - 1) {
                        output_mapping.insert("out-0".to_string(), format!("f{}", (i + 1)));
                    } else if *inter_arrival == 0 {
                        assert!(i == (chain_size - 1));
                        output_mapping.insert("out-0".to_string(), "f0".to_string());
                    }
                    let matrix_size: u32 = self.rng.gen_range(*min_matrix_size..=*max_matrix_size);
                    matrix_sizes.push(matrix_size);

                    let annotations = std::collections::HashMap::from([(
                        "init-payload".to_string(),
                        format!(
                            "seed={},inter_arrival={},is_first={},is_last={},wf_name=wf{},fun_name=f{},matrix_size={},outputs=0",
                            i,
                            inter_arrival,
                            match i {
                                0 => "true",
                                _ => "false",
                            },
                            match chain_size - 1 - i {
                                0 => "true",
                                _ => "false",
                            },
                            self.wf_id,
                            i,
                            matrix_size
                        )
                        .to_string(),
                    )]);

                    functions.push(WorkflowFunction {
                        name: format!("f{}", i).to_string(),
                        function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                            function_class_id: "matrix_mul".to_string(),
                            function_class_type: "RUST_WASM".to_string(),
                            function_class_version: "0.1".to_string(),
                            function_class_inlude_code: std::fs::read(path_wasm).unwrap(),
                            outputs,
                        },
                        output_mapping,
                        annotations,
                    });
                }

                log::info!(
                    "wf{}, chain size {}, matrix sizes {}",
                    self.wf_id,
                    chain_size,
                    matrix_sizes.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",")
                );

                resources.push(edgeless_api::workflow_instance::WorkflowResource {
                    name: "metrics-collector".to_string(),
                    class_type: "metrics-collector".to_string(),
                    output_mapping: std::collections::HashMap::new(),
                    configurations: std::collections::HashMap::from([("url".to_string(), redis_url.to_string())]),
                })
            }
        }

        self.wf_id += 1;

        if functions.is_empty() {
            assert!(resources.is_empty());
            return Ok("".to_string());
        }

        let res = self
            .client
            .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
                workflow_functions: functions,
                workflow_resources: resources,
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

    // Parse the worflow type from command line option.
    let wf_type = match workflow_type(&args.wf_type) {
        Ok(val) => val,
        Err(err) => {
            return Err(anyhow::anyhow!("invalid workflow type: {}", err));
        }
    };

    // Parse the arrival model.
    let arrival_model = match args.arrival_model.as_str() {
        "poisson" => ArrivalModel::Poisson,
        "incremental" => ArrivalModel::Incremental,
        _ => panic!("unknown arrival model {}: ", args.arrival_model),
    };

    // Start the metrics collector node, if needed
    if let WorkflowType::MatrixMulChain(_, _, _, _, _, _, _) = wf_type {
        let _ = tokio::spawn(async move {
            edgeless_benchmark::edgeless_metrics_collector_node_main(edgeless_node::EdgelessNodeSettings {
                node_id: uuid::Uuid::new_v4(),
                agent_url: format!("http://{}:7121/", args.bind_address),
                invocation_url: format!("http://{}:7102/", args.bind_address),
                metrics_url: format!("http://{}:7103/", args.bind_address),
                orchestrator_url: args.orchestrator_url,
                http_ingress_url: "".to_string(),
                http_ingress_provider: "".to_string(),
                http_egress_provider: "".to_string(),
                file_log_provider: "".to_string(),
                redis_provider: "".to_string(),
            })
            .await
        });
    }

    // Create an e-ORC client
    let mut client_interface = ClientInterface::new(&args.controller_url, wf_type).await;

    // event queue, the first event is always a new workflow arriving at time 0
    let mut events = BinaryHeap::new();
    events.push(Event::WfNew(0_u64)); // in us

    // add the end-of-experiment event
    events.push(Event::WfExperimentEnd(to_microseconds(args.duration)));

    // main experiment loop
    let mut wf_started = 0;
    let mut wf_requested = 0;
    let mut now = 0;
    'outer: loop {
        if let Some(event) = events.pop() {
            // wait until the event
            assert!(event.time() >= now);
            if event.time() > now {
                std::thread::sleep(time::Duration::from_micros(event.time() - now));
            }

            // handle the event
            now = event.time();
            match event {
                Event::WfNew(_) => {
                    wf_requested += 1;
                    match client_interface.start_workflow().await {
                        Ok(uuid) => {
                            wf_started += 1;
                            let end_time = match arrival_model {
                                ArrivalModel::Poisson => now + to_microseconds(lifetime_rv.sample(&mut rng)),
                                ArrivalModel::Incremental => to_microseconds(args.duration) - 1,
                            };
                            assert!(end_time >= now);
                            log::info!(
                                "{} new wf created '{}', will last {} s",
                                to_seconds(now),
                                &uuid,
                                to_seconds(end_time - now)
                            );
                            events.push(Event::WfEnd(end_time, uuid));
                        }
                        Err(_) => {}
                    }
                    let new_arrival_time = now
                        + to_microseconds(match arrival_model {
                            ArrivalModel::Poisson => interarrival_rv.sample(&mut rng),
                            ArrivalModel::Incremental => args.interarrival,
                        });
                    if new_arrival_time < to_microseconds(args.duration) {
                        // only add the event if it is before the end of the experiment
                        events.push(Event::WfNew(new_arrival_time));
                    }
                }
                Event::WfEnd(_, uuid) => {
                    log::info!("{} wf terminated  '{}'", to_seconds(now), &uuid);
                    if !uuid.is_empty() {
                        match client_interface.stop_workflow(&uuid).await {
                            Ok(_) => {}
                            Err(err) => {
                                panic!("error when stopping a workflow: {}", err);
                            }
                        }
                    }
                }
                Event::WfExperimentEnd(_) => {
                    break 'outer;
                }
            }
        }
    }

    // terminate all workflows that are still active
    for event_type in events.iter() {
        if let Event::WfEnd(_, uuid) = event_type {
            if !uuid.is_empty() {
                match client_interface.stop_workflow(uuid).await {
                    Ok(_) => {}
                    Err(err) => {
                        panic!("error when stopping a workflow: {}", err);
                    }
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
