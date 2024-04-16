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
    /// Arrival model, one of {poisson, incremental, incr-and-keep}
    #[arg(long, default_value_t = String::from("poisson"))]
    arrival_model: String,
    /// Warmup duration, in s
    #[arg(long, default_value_t = 10.0)]
    warmup: f64,
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
    /// URL of the Redis server to use for metrics.
    #[arg(short, long, default_value_t = String::from("redis://127.0.0.1:6379/"))]
    redis_url: String,
    /// Name of the CSV output file where to save the metrics collected.
    #[arg(long, default_value_t = String::from("out.csv"))]
    output: String,
    /// Append to the output file.
    #[arg(long, default_value_t = false)]
    append: bool,
    /// Additional fields recorded in the CSV output file.
    #[arg(long, default_value_t = String::from(""))]
    additional_fields: String,
    /// Header of additional fields recorded in the CSV output file.
    #[arg(long, default_value_t = String::from(""))]
    additional_header: String,
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
    /// 0: Event time.
    WfWarmUpEnd(u64),
}

impl Event {
    fn time(&self) -> u64 {
        match self {
            Self::WfNew(t) => *t,
            Self::WfEnd(t, _) => *t,
            Self::WfExperimentEnd(t) => *t,
            Self::WfWarmUpEnd(t) => *t,
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
    /// Add workflows incrementally until the warm up period finishes, then keep until the end of the experiment.
    IncrAndKeep,
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
    // Example: single;examples/noop/noop_function/function.json;examples/noop/noop_function/noop.wasm
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
    // Example: matrix-mul-chain;5;5;1000;2000;1000;edgeless_benchmark/functions/vector_mul/matrix_mul.wasm
    MatrixMulChain(u32, u32, u32, u32, u32, String),
    // 0: min chain length
    // 1: max chain length
    // 2: min input size
    // 3: max input size
    // 4: vector_mul.wasm path
    // Example: vector-mul-chain;5;5;1000;2000;edgeless_benchmark/functions/vector_mul/vector_mul.wasm
    VectorMulChain(u32, u32, u32, u32, String),
}

impl WorkflowType {
    fn metrics_collector(&self) -> bool {
        match self {
            Self::None => true,
            Self::Single(_, _) => false,
            Self::MatrixMulChain(_, _, _, _, _, _) => true,
            Self::VectorMulChain(_, _, _, _, _) => true,
        }
    }
}

fn workflow_type(wf_type: &str) -> anyhow::Result<WorkflowType> {
    let tokens: Vec<&str> = wf_type.split(';').collect();
    if !tokens.is_empty() && tokens[0] == "none" {
        return Ok(WorkflowType::None);
    } else if !tokens.is_empty() && tokens[0] == "single" && tokens.len() == 3 {
        return Ok(WorkflowType::Single(tokens[1].to_string(), tokens[2].to_string()));
    } else if !tokens.is_empty() && tokens[0] == "matrix-mul-chain" && tokens.len() == 7 {
        return Ok(WorkflowType::MatrixMulChain(
            tokens[1].parse::<u32>().unwrap_or_default(),
            tokens[2].parse::<u32>().unwrap_or_default(),
            tokens[3].parse::<u32>().unwrap_or_default(),
            tokens[4].parse::<u32>().unwrap_or_default(),
            tokens[5].parse::<u32>().unwrap_or_default(),
            tokens[6].to_string(),
        ));
    } else if !tokens.is_empty() && tokens[0] == "vector-mul-chain" && tokens.len() == 6 {
        return Ok(WorkflowType::VectorMulChain(
            tokens[1].parse::<u32>().unwrap_or_default(),
            tokens[2].parse::<u32>().unwrap_or_default(),
            tokens[3].parse::<u32>().unwrap_or_default(),
            tokens[4].parse::<u32>().unwrap_or_default(),
            tokens[5].to_string(),
        ));
    }
    Err(anyhow!("unknown workflow type: {}", wf_type))
}

struct ClientInterface {
    client: Box<dyn WorkflowInstanceAPI>,
    redis_url: String,
    wf_type: WorkflowType,
    rng: rand::rngs::StdRng,
    /// Identifier of the next workflow to start.
    wf_id: u32,
    /// Workflows started. For each workflow, functions started.
    workflows: std::collections::HashMap<String, std::collections::HashSet<String>>,
}

impl ClientInterface {
    async fn new(controller_url: &str, redis_url: &str, wf_type: WorkflowType) -> Self {
        Self {
            client: edgeless_api::grpc_impl::controller::ControllerAPIClient::new(controller_url)
                .await
                .workflow_instance_api(),
            redis_url: redis_url.to_string(),
            wf_type,
            rng: rand::rngs::StdRng::from_entropy(),
            wf_id: 0,
            workflows: std::collections::HashMap::new(),
        }
    }

    async fn start_workflow(&mut self) -> anyhow::Result<String> {
        let mut functions = vec![];
        let mut resources: Vec<edgeless_api::workflow_instance::WorkflowResource> = vec![];

        let mut function_names = std::collections::HashSet::new();
        let wf_name = format!("wf{}", self.wf_id);

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
                        function_class_code: std::fs::read(path_wasm).unwrap(),
                        function_class_outputs: func_spec.outputs,
                    },
                    output_mapping: std::collections::HashMap::new(),
                    annotations: std::collections::HashMap::new(),
                });
            }
            WorkflowType::MatrixMulChain(min_chain_size, max_chain_size, min_matrix_size, max_matrix_size, inter_arrival, path_wasm) => {
                let chain_size: u32 = self.rng.gen_range(*min_chain_size..=*max_chain_size);

                let mut matrix_sizes = vec![];

                for i in 0..chain_size {
                    let mut outputs = vec!["metric".to_string()];
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

                    let name = format!("f{}", i);
                    let annotations = std::collections::HashMap::from([(
                        "init-payload".to_string(),
                        format!(
                            "seed={},inter_arrival={},is_first={},is_last={},wf_name={},fun_name={},matrix_size={},outputs=0",
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
                            &wf_name,
                            &name,
                            matrix_size
                        )
                        .to_string(),
                    )]);

                    function_names.insert(name.clone());
                    functions.push(WorkflowFunction {
                        name,
                        function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                            function_class_id: "matrix_mul".to_string(),
                            function_class_type: "RUST_WASM".to_string(),
                            function_class_version: "0.1".to_string(),
                            function_class_code: std::fs::read(path_wasm).unwrap(),
                            function_class_outputs: outputs,
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
                    configurations: std::collections::HashMap::from([("url".to_string(), self.redis_url.clone())]),
                });
            }
            WorkflowType::VectorMulChain(min_chain_size, max_chain_size, min_input_size, max_input_size, path_wasm) => {
                let chain_size: u32 = self.rng.gen_range(*min_chain_size..=*max_chain_size);
                let input_size = self.rng.gen_range(*min_input_size..=*max_input_size);

                for i in 0..chain_size {
                    let name = match i {
                        0 => "client".to_string(),
                        i => format!("f{}", i - 1),
                    };
                    let next_func_name = match chain_size - i - 1 {
                        0 => "client".to_string(),
                        i => format!("f{}", chain_size - i - 1),
                    };

                    let output_mapping = std::collections::HashMap::from([
                        ("metric".to_string(), "metrics-collector".to_string()),
                        ("out".to_string(), next_func_name),
                    ]);

                    let annotations = std::collections::HashMap::from([(
                        "init-payload".to_string(),
                        format!(
                            "seed={},is_client={},wf_name={},fun_name={},input_size={}",
                            i,
                            match i {
                                0 => "true",
                                _ => "false",
                            },
                            &wf_name,
                            &name,
                            input_size
                        )
                        .to_string(),
                    )]);

                    if i > 0 {
                        function_names.insert(name.clone());
                    }
                    functions.push(WorkflowFunction {
                        name,
                        function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                            function_class_id: "vector_mul".to_string(),
                            function_class_type: "RUST_WASM".to_string(),
                            function_class_version: "0.1".to_string(),
                            function_class_code: std::fs::read(path_wasm).unwrap(),
                            function_class_outputs: vec!["metric".to_string(), "out".to_string()],
                        },
                        output_mapping,
                        annotations,
                    });
                }

                log::info!("wf{}, chain size {}, input size {}", self.wf_id, chain_size, input_size);

                resources.push(edgeless_api::workflow_instance::WorkflowResource {
                    name: "metrics-collector".to_string(),
                    class_type: "metrics-collector".to_string(),
                    output_mapping: std::collections::HashMap::new(),
                    configurations: std::collections::HashMap::from([("url".to_string(), self.redis_url.clone())]),
                });
            }
        };

        self.wf_id += 1;

        if functions.is_empty() {
            assert!(resources.is_empty());
            return Ok("".to_string());
        }

        self.workflows.insert(wf_name, function_names);

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
        "incr-and-keep" => ArrivalModel::IncrAndKeep,
        _ => panic!("unknown arrival model {}: ", args.arrival_model),
    };

    // Check that the additional fields, if present, have a consistent header.
    let mut additional_fields = args.additional_fields.split(',').filter(|x| !x.is_empty()).collect::<Vec<&str>>();
    let mut additional_header = args.additional_header.split(',').filter(|x| !x.is_empty()).collect::<Vec<&str>>();
    if additional_fields.len() != additional_header.len() {
        return Err(anyhow::anyhow!(
            "mismatching number of additional fields ({}) vs header ({})",
            additional_fields.len(),
            additional_header.len()
        ));
    }
    let seed = format!("{}", args.seed);
    additional_fields.push(&seed);
    additional_header.push("seed");

    // Start the metrics collector node, if needed
    let mut redis_client =
        edgeless_benchmark::redis_dumper::RedisDumper::new(&args.redis_url, additional_fields.join(","), additional_header.join(","));
    if redis_client.is_ok() {
        log::info!("connected to Redis at {}", &args.redis_url);
    }
    if wf_type.metrics_collector() {
        let _ = tokio::spawn(async move {
            edgeless_benchmark::edgeless_metrics_collector_node_main(edgeless_node::EdgelessNodeGeneralSettings {
                node_id: uuid::Uuid::new_v4(),
                agent_url: format!("http://{}:7121/", args.bind_address),
                agent_url_announced: "".to_string(),
                invocation_url: format!("http://{}:7102/", args.bind_address),
                invocation_url_announced: "".to_string(),
                metrics_url: format!("http://{}:7103/", args.bind_address),
                orchestrator_url: args.orchestrator_url,
            })
            .await
        });
        if let Err(err) = &redis_client {
            log::warn!("could not connect to Redis at {}: {}", &args.redis_url, err);
        }
    }

    // Create an e-ORC client
    let mut client_interface = ClientInterface::new(&args.controller_url, &args.redis_url, wf_type).await;

    // event queue, the first event is always a new workflow arriving at time 0
    let mut events = BinaryHeap::new();
    events.push(Event::WfNew(0_u64)); // in us

    // add the end-of-experiment event
    events.push(Event::WfExperimentEnd(to_microseconds(args.duration)));

    // add the event for the end of the warm-up period
    if args.warmup < args.duration {
        events.push(Event::WfWarmUpEnd(to_microseconds(args.warmup)));
    } else {
        log::warn!(
            "metrics will not be collected since warm-up period ({} s) >= experiment duration ({} s)",
            args.warmup,
            args.duration
        );
    }

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
                    // do not schedule any more workflows after the warm-up period is finished
                    // for IncrAndKeep arrival model
                    if now >= to_microseconds(args.warmup) {
                        continue;
                    }

                    wf_requested += 1;
                    match client_interface.start_workflow().await {
                        Ok(uuid) => {
                            wf_started += 1;
                            let end_time = match arrival_model {
                                ArrivalModel::Poisson => now + to_microseconds(lifetime_rv.sample(&mut rng)),
                                ArrivalModel::Incremental | ArrivalModel::IncrAndKeep => to_microseconds(args.duration) - 1,
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
                            ArrivalModel::Incremental | ArrivalModel::IncrAndKeep => args.interarrival,
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
                Event::WfWarmUpEnd(_) => {
                    if let Ok(client) = &mut redis_client {
                        let _ = client.flushdb();
                    }
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

    // dump data collected in Redis
    if let Ok(client) = &mut redis_client {
        if let Err(err) = client.dump_csv(&args.output, args.append, client_interface.workflows) {
            log::error!("error dumping to {}: {}", args.output, err);
        }
    }

    // output metrics
    let blocking_probability = 1.0 - wf_started as f64 / wf_requested as f64;

    log::info!("workflow requested   = {}", wf_requested);
    log::info!("workflow started     = {}", wf_started);
    log::info!("blocking probability = {}", blocking_probability);

    Ok(())
}
