// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use anyhow::anyhow;
use clap::Parser;
use core::cmp::Ordering;
use edgeless_api::controller::ControllerAPI;
use edgeless_api::workflow_instance::{SpawnWorkflowResponse, WorkflowFunction, WorkflowId, WorkflowInstanceAPI};
use edgeless_benchmark::redis_dumper;
use rand::prelude::*;
use rand::SeedableRng;
use rand_distr::Exp;
use rand_pcg::Pcg64;
use std::collections::BinaryHeap;
use std::time;

const ALPHA: f64 = 0.9_f64;

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
    /// Arrival model, one of {poisson, incremental, incr-and-keep, single}
    #[arg(long, default_value_t = String::from("poisson"))]
    arrival_model: String,
    /// Warmup duration, in s
    #[arg(long, default_value_t = 0.0)]
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
    /// Workflow type, use "help" to list possible examples.
    #[arg(short, long, default_value_t = String::from("single;functions/noop/function.json;functions/noop/noop.wasm"))]
    wf_type: String,
    /// Location of the single_trigger function.
    #[arg(long, default_value_t = String::from("functions/single_trigger/single_trigger.wasm"))]
    single_trigger_wasm: String,
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
    /// Add workflows incrementally until the warm up period finishes, then keep until the end of the experiment.
    IncrAndKeep,
    /// Add a single workflow.
    Single,
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
    MatrixMulChain(u32, u32, u32, u32, u32, String),
    // 0: min chain length
    // 1: max chain length
    // 2: min input size
    // 3: max input size
    // 4: vector_mul.wasm path
    VectorMulChain(u32, u32, u32, u32, String),
    // 0:  min interval between consecutive transactions, in ms
    // 1:  max interval between consecutive transactions, in ms
    // 2:  min input vector size
    // 3:  min input vector size
    // 4:  min number of stages
    // 5:  max number of stages
    // 6:  min fan-out per stage
    // 7:  max fan-out per stage
    // 8:  min element of the Fibonacci sequence to compute
    // 9:  max element of the Fibonacci sequence to compute
    // 10: min memory allocation, in bytes
    // 11: max memory allocation, in bytes
    // 12: base path of the functions library
    MapReduce(u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, String),
}

impl WorkflowType {
    fn new(wf_type: &str) -> anyhow::Result<Self> {
        let tokens: Vec<&str> = wf_type.split(';').collect();
        if !tokens.is_empty() && tokens[0] == "none" {
            return WorkflowType::None.check();
        } else if !tokens.is_empty() && tokens[0] == "single" && tokens.len() == 3 {
            return WorkflowType::Single(tokens[1].to_string(), tokens[2].to_string()).check();
        } else if !tokens.is_empty() && tokens[0] == "matrix-mul-chain" && tokens.len() == 7 {
            return WorkflowType::MatrixMulChain(
                tokens[1].parse::<u32>().unwrap_or_default(),
                tokens[2].parse::<u32>().unwrap_or_default(),
                tokens[3].parse::<u32>().unwrap_or_default(),
                tokens[4].parse::<u32>().unwrap_or_default(),
                tokens[5].parse::<u32>().unwrap_or_default(),
                tokens[6].to_string(),
            )
            .check();
        } else if !tokens.is_empty() && tokens[0] == "vector-mul-chain" && tokens.len() == 6 {
            return WorkflowType::VectorMulChain(
                tokens[1].parse::<u32>().unwrap_or_default(),
                tokens[2].parse::<u32>().unwrap_or_default(),
                tokens[3].parse::<u32>().unwrap_or_default(),
                tokens[4].parse::<u32>().unwrap_or_default(),
                tokens[5].to_string(),
            )
            .check();
        } else if !tokens.is_empty() && tokens[0] == "map-reduce" && tokens.len() == 14 {
            return WorkflowType::MapReduce(
                tokens[1].parse::<u32>().unwrap_or_default(),
                tokens[2].parse::<u32>().unwrap_or_default(),
                tokens[3].parse::<u32>().unwrap_or_default(),
                tokens[4].parse::<u32>().unwrap_or_default(),
                tokens[5].parse::<u32>().unwrap_or_default(),
                tokens[6].parse::<u32>().unwrap_or_default(),
                tokens[7].parse::<u32>().unwrap_or_default(),
                tokens[8].parse::<u32>().unwrap_or_default(),
                tokens[9].parse::<u32>().unwrap_or_default(),
                tokens[10].parse::<u32>().unwrap_or_default(),
                tokens[11].parse::<u32>().unwrap_or_default(),
                tokens[12].parse::<u32>().unwrap_or_default(),
                tokens[13].to_string(),
            )
            .check();
        }
        Err(anyhow!("unknown workflow type: {}", wf_type))
    }

    fn check(self) -> anyhow::Result<Self> {
        match &self {
            WorkflowType::None => {}
            WorkflowType::Single(json, wasm) => {
                anyhow::ensure!(!json.is_empty(), "empty JSON file path");
                anyhow::ensure!(!wasm.is_empty(), "empty WASM file path");
            }
            WorkflowType::VectorMulChain(min_chain, max_chain, min_size, max_size, wasm) => {
                anyhow::ensure!(*min_chain > 0, "vanishing min chain");
                anyhow::ensure!(max_chain >= min_chain, "chain: min > max");
                anyhow::ensure!(max_size >= min_size, "size: min > max");
                anyhow::ensure!(!wasm.is_empty(), "empty WASM file path");
            }
            WorkflowType::MatrixMulChain(min_chain, max_chain, min_size, max_size, _interval, wasm) => {
                anyhow::ensure!(*min_chain > 0, "vanishing min chain");
                anyhow::ensure!(max_chain >= min_chain, "chain: min > max");
                anyhow::ensure!(max_size >= min_size, "size: min > max");
                anyhow::ensure!(!wasm.is_empty(), "empty WASM file path");
            }
            WorkflowType::MapReduce(
                min_interval,
                max_interval,
                min_size,
                max_size,
                min_stages,
                max_stages,
                min_breadth,
                max_breadth,
                min_fibonacci,
                max_fibonacci,
                min_allocate,
                max_allocate,
                library_path,
            ) => {
                anyhow::ensure!(*min_interval > 0, "vanishing min interval");
                anyhow::ensure!(max_interval >= min_interval, "interval: min > max");
                anyhow::ensure!(max_size >= min_size, "rate: min > max");
                anyhow::ensure!(*min_stages > 0, "vanishing min stages");
                anyhow::ensure!(max_stages >= min_stages, "rate: min > max");
                anyhow::ensure!(*min_breadth > 0, "vanishing min rate");
                anyhow::ensure!(max_breadth >= min_breadth, "breadth: min > max");
                anyhow::ensure!(max_fibonacci >= min_fibonacci, "fibonacci: min > max");
                anyhow::ensure!(max_allocate >= min_allocate, "allocation: min > max");
                anyhow::ensure!(!library_path.is_empty(), "empty library path");
            }
        }
        Ok(self)
    }

    fn metrics_collector(&self) -> bool {
        match self {
            WorkflowType::None | WorkflowType::Single(_, _) => false,
            _ => true,
        }
    }

    fn examples() -> Vec<Self> {
        vec![
            WorkflowType::None,
            WorkflowType::Single("functions/noop/function.json".to_string(), "functions/noop/noop.wasm".to_string()),
            WorkflowType::VectorMulChain(3, 5, 1000, 1000, "functions/vector_mul/vector_mul.wasm".to_string()),
            WorkflowType::MatrixMulChain(3, 5, 100, 200, 1000, "functions/matrix_mul/matrix_mul.wasm".to_string()),
            WorkflowType::MapReduce(1000, 1000, 500, 500, 3, 3, 2, 2, 10000, 10000, 0, 0, "functions/".to_string()),
        ]
    }
}

impl std::fmt::Display for WorkflowType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WorkflowType::None => write!(f, "none"),
            WorkflowType::Single(json, wasm) => write!(f, "single;{};{}", json, wasm),
            WorkflowType::VectorMulChain(min_chain, max_chain, min_size, max_size, wasm) => {
                write!(f, "vector-mul-chain;{};{};{};{};{}", min_chain, max_chain, min_size, max_size, wasm)
            }
            WorkflowType::MatrixMulChain(min_chain, max_chain, min_size, max_size, interval, wasm) => {
                write!(
                    f,
                    "matrix-mul-chain;{};{};{};{};{};{}",
                    min_chain, max_chain, min_size, max_size, interval, wasm
                )
            }
            WorkflowType::MapReduce(
                min_interval,
                max_interval,
                min_size,
                max_size,
                min_stages,
                max_stages,
                min_breadth,
                max_breadth,
                min_fibonacci,
                max_fibonacci,
                min_allocate,
                max_allocate,
                library_path,
            ) => {
                write!(
                    f,
                    "map-reduce;{};{};{};{};{};{};{};{};{};{};{};{};{}",
                    min_interval,
                    max_interval,
                    min_size,
                    max_size,
                    min_stages,
                    max_stages,
                    min_breadth,
                    max_breadth,
                    min_fibonacci,
                    max_fibonacci,
                    min_allocate,
                    max_allocate,
                    library_path,
                )
            }
        }
    }
}

struct ClientInterface {
    /// The client interface.
    client: Box<dyn WorkflowInstanceAPI>,
    /// Type of workflows generated.
    wf_type: WorkflowType,
    /// Pseudo-random number generator.
    rng: rand::rngs::StdRng,
    /// Identifier of the next workflow to start.
    wf_id: u32,
    /// Redis client.
    redis_client: Option<edgeless_benchmark::redis_dumper::RedisDumper>,
}

async fn setup_metrics_collector(client_interface: &mut ClientInterface, single_trigger_wasm: &str, warmup: f64) -> anyhow::Result<String> {
    let function_class_code = match std::fs::read(single_trigger_wasm) {
        Ok(code) => code,
        Err(err) => anyhow::bail!("cannot read source: {}", err),
    };
    let res = client_interface
        .client
        .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
            workflow_functions: vec![WorkflowFunction {
                name: "single_trigger".to_string(),
                function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                    function_class_id: "matrix_mul".to_string(),
                    function_class_type: "RUST_WASM".to_string(),
                    function_class_version: "0.1".to_string(),
                    function_class_code,
                    function_class_outputs: vec!["out".to_string()],
                },
                output_mapping: std::collections::HashMap::from([("out".to_string(), "metrics-collector".to_string())]),
                annotations: std::collections::HashMap::from([("init-payload".to_string(), format!("reset:{}", (warmup * 1000.0) as u64))]),
            }],
            workflow_resources: vec![edgeless_api::workflow_instance::WorkflowResource {
                name: "metrics-collector".to_string(),
                class_type: "metrics-collector".to_string(),
                output_mapping: std::collections::HashMap::new(),
                configurations: std::collections::HashMap::new(),
            }],
            annotations: std::collections::HashMap::new(),
        })
        .await;
    match res {
        Ok(response) => match &response {
            SpawnWorkflowResponse::ResponseError(err) => Err(anyhow!("{}", err)),
            SpawnWorkflowResponse::WorkflowInstance(val) => Ok(val.workflow_id.workflow_id.to_string()),
        },
        Err(err) => {
            panic!("error when setting up warm-up on the metrics collector: {}", err);
        }
    }
}

impl ClientInterface {
    async fn new(
        controller_url: &str,
        wf_type: WorkflowType,
        seed: u64,
        redis_client: Option<edgeless_benchmark::redis_dumper::RedisDumper>,
    ) -> Self {
        Self {
            client: edgeless_api::grpc_impl::controller::ControllerAPIClient::new(controller_url)
                .await
                .workflow_instance_api(),
            wf_type,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
            wf_id: 0,
            redis_client,
        }
    }

    fn dump(&mut self, output: &str, append: bool) {
        if let Some(redis_client) = &mut self.redis_client {
            if let Err(err) = redis_client.dump_csv(output, append) {
                log::error!("error dumping from Redis to file {}: {}", output, err);
            }
        }
    }

    async fn start_workflow(&mut self) -> anyhow::Result<String> {
        let mut functions = vec![];
        let mut resources: Vec<edgeless_api::workflow_instance::WorkflowResource> = vec![];

        let wf_name = format!("wf{}", self.wf_id);

        let mut draw = |lower: u32, higher: u32| {
            assert!(lower <= higher);
            if lower == higher {
                lower
            } else {
                self.rng.gen_range(lower..=higher)
            }
        };

        let to_true_false = |val: bool| {
            if val {
                "true"
            } else {
                "false"
            }
        };

        let function_class_specification = |path_json: &std::path::Path, path_wasm: &std::path::Path| {
            let func_spec: edgeless_cli::workflow_spec::WorkflowSpecFunctionClass =
                serde_json::from_str(&std::fs::read_to_string(path_json).unwrap()).unwrap();
            edgeless_api::function_instance::FunctionClassSpecification {
                function_class_id: func_spec.id,
                function_class_type: func_spec.function_type,
                function_class_version: func_spec.version,
                function_class_code: std::fs::read(path_wasm).unwrap(),
                function_class_outputs: func_spec.outputs,
            }
        };

        match &self.wf_type {
            WorkflowType::None => {}
            WorkflowType::Single(path_json, path_wasm) => {
                functions.push(WorkflowFunction {
                    name: "single".to_string(),
                    function_class_specification: function_class_specification(std::path::Path::new(path_json), std::path::Path::new(path_wasm)),
                    output_mapping: std::collections::HashMap::new(),
                    annotations: std::collections::HashMap::new(),
                });
            }
            WorkflowType::MatrixMulChain(min_chain_size, max_chain_size, min_matrix_size, max_matrix_size, inter_arrival, path_wasm) => {
                let chain_size: u32 = draw(*min_chain_size, *max_chain_size);

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
                    let matrix_size = draw(*min_matrix_size, *max_matrix_size);
                    matrix_sizes.push(matrix_size);

                    let name = format!("f{}", i);
                    let annotations = std::collections::HashMap::from([(
                        "init-payload".to_string(),
                        format!(
                            "seed={},inter_arrival={},is_first={},is_last={},matrix_size={},outputs={}",
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
                            matrix_size,
                            match chain_size - 1 - i {
                                0 => "",
                                _ => "0",
                            },
                        ),
                    )]);
                    log::debug!("name {}, annotations {:?} mapping {:?}", name, annotations, output_mapping);

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
            }
            WorkflowType::VectorMulChain(min_chain_size, max_chain_size, min_input_size, max_input_size, path_wasm) => {
                let chain_size = draw(*min_chain_size, *max_chain_size);
                let input_size = draw(*min_input_size, *max_input_size);

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
                            "seed={},is_client={},input_size={}",
                            i,
                            match i {
                                0 => "true",
                                _ => "false",
                            },
                            input_size
                        )
                        .to_string(),
                    )]);

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
            }
            WorkflowType::MapReduce(
                min_interval,
                max_interval,
                min_size,
                max_size,
                min_stages,
                max_stages,
                min_breadth,
                max_breadth,
                min_fibonacci,
                max_fibonacci,
                min_allocate,
                max_allocate,
                library_path,
            ) => {
                //
                //
                //
                //                                     ┌─────────┐                         ┌─────────┐
                //                                     │         │                         │         │
                //                         ┌──────────►│  p0-0   ├──────────┐  ┌──────────►│  p1-0   ├──────────┐
                //                         │           │         │          │  │           │         │          │
                //                         │           └─────────┘          │  │           └─────────┘          │
                //                         │                                ▼  │                                ▼
                // ┌─────────┐       ┌─────┴────┐      ┌─────────┐       ┌─────┴────┐      ┌─────────┐       ┌─────────┐
                // │         │       │          │      │         │       │          │      │         │       │         │
                // │ trigger │──────►│    s0    ├─────►│  p0-1   ├──────►│   s1     ├─────►│  p1-1   ├──────►│   s2    │
                // │         │       │          │      │         │       │          │      │         │       │         │
                // └─────────┘       └─────┬────┘      └─────────┘       └─────┬────┘      └─────────┘       └─────────┘
                //                         │                                ▲  │                                ▲
                //                         │           ┌─────────┐          │  │           ┌─────────┐          │
                //                         │           │         │          │  │           │         │          │
                //                         └──────────►│  p0-2   ├──────────┘  └──────────►│  p1-2   ├──────────┘
                //                                     │         │                         │         │
                //                                     └─────────┘                         └─────────┘
                let interval = draw(*min_interval, *max_interval);
                let size = draw(*min_size, *max_size);
                let stages = draw(*min_stages, *max_stages);

                let path = std::path::Path::new(library_path);

                functions.push(WorkflowFunction {
                    name: "trigger".to_string(),
                    function_class_specification: function_class_specification(
                        path.join("trigger/function.json").as_path(),
                        path.join("trigger/trigger.wasm").as_path(),
                    ),
                    output_mapping: std::collections::HashMap::from([("out".to_string(), "s0".to_string())]),
                    annotations: std::collections::HashMap::from([(
                        "init-payload".to_string(),
                        format!("out_type=rand_vec,use_base64=true,size={},arrival=c({})", size, interval),
                    )]),
                });

                let mut inputs: Vec<u32> = vec![];
                let mut breadths = vec![];
                let mut fibonacci_values = vec![];
                let mut allocate_values = vec![];
                for stage in 0..=stages {
                    let breadth = draw(*min_breadth, *max_breadth);
                    let first = stage == 0;
                    let last = stage == stages;
                    let outputs: Vec<u32> = if last { vec![] } else { (0..breadth).collect() };
                    breadths.push(outputs.len());
                    let mut output_mapping = std::collections::HashMap::new();
                    if first || last {
                        output_mapping.insert("metric".to_string(), "metrics-collector".to_string());
                    }
                    for out in &outputs {
                        output_mapping.insert(format!("out-{}", out), format!("p{}-{}", stage, out));
                    }
                    functions.push(WorkflowFunction {
                        name: format!("s{}", stage),
                        function_class_specification: function_class_specification(
                            path.join("bench_mapreduce/function.json").as_path(),
                            path.join("bench_mapreduce/bench_mapreduce.wasm").as_path(),
                        ),
                        output_mapping,
                        annotations: std::collections::HashMap::from([(
                            "init-payload".to_string(),
                            format!(
                                "is_first={},is_last={},use_base64=true,inputs={},outputs={}",
                                to_true_false(first),
                                to_true_false(last),
                                inputs.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(":"),
                                outputs.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(":")
                            ),
                        )]),
                    });

                    fibonacci_values.push(vec![]);
                    allocate_values.push(vec![]);
                    for out in &outputs {
                        let fibonacci = draw(*min_fibonacci, *max_fibonacci);
                        fibonacci_values.last_mut().unwrap().push(fibonacci);
                        let allocate = draw(*min_allocate, *max_allocate);
                        allocate_values.last_mut().unwrap().push(allocate);
                        functions.push(WorkflowFunction {
                            name: format!("p{}-{}", stage, out),
                            function_class_specification: function_class_specification(
                                path.join("bench_process/function.json").as_path(),
                                path.join("bench_process/bench_process.wasm").as_path(),
                            ),
                            output_mapping: std::collections::HashMap::from([("out".to_string(), format!("s{}", stage + 1))]),
                            annotations: std::collections::HashMap::from([(
                                "init-payload".to_string(),
                                format!("forward=true,fibonacci={},allocate={}", fibonacci, allocate),
                            )]),
                        });
                    }

                    inputs = outputs;
                }
                log::info!(
                    "wf{}, average interval {} ms, input size {}, num stages {}, breadths [{}], fibonacci [{}], allocate [{}]",
                    self.wf_id,
                    interval,
                    size,
                    stages,
                    breadths.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","),
                    fibonacci_values
                        .iter()
                        .map(|x| format!("[{}]", x.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",")))
                        .collect::<Vec<String>>()
                        .join(","),
                    allocate_values
                        .iter()
                        .map(|x| format!("[{}]", x.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",")))
                        .collect::<Vec<String>>()
                        .join(",")
                );
            }
        };

        if self.wf_type.metrics_collector() {
            resources.push(edgeless_api::workflow_instance::WorkflowResource {
                name: "metrics-collector".to_string(),
                class_type: "metrics-collector".to_string(),
                output_mapping: std::collections::HashMap::new(),
                configurations: std::collections::HashMap::from([
                    ("alpha".to_string(), format!("{}", ALPHA)),
                    ("wf_name".to_string(), wf_name.clone()),
                ]),
            });
        }

        self.wf_id += 1;

        if functions.is_empty() {
            assert!(resources.is_empty());
            return Ok("".to_string());
        }

        // Prepare the workflow creation request and save it to the Redis
        // JSON-serialized in the following key:
        // workflow:$wf_name:request
        let req = edgeless_api::workflow_instance::SpawnWorkflowRequest {
            workflow_functions: functions,
            workflow_resources: resources,
            annotations: std::collections::HashMap::new(),
        };
        if let Some(redis_client) = &mut self.redis_client {
            redis_client.set(
                format!("workflow:{}:request", wf_name).as_str(),
                serde_json::to_string(&req).unwrap_or_default().as_str(),
            );
        }

        // Request the creation of the workflow.
        let res = self.client.start(req).await;

        // Save the JSON-serialized response to Redis in the following key:
        // workflow:$wf_name:response
        match res {
            Ok(response) => {
                if let Some(redis_client) = &mut self.redis_client {
                    redis_client.set(
                        format!("workflow:{}:response", wf_name).as_str(),
                        serde_json::to_string(&response).unwrap_or_default().as_str(),
                    );
                }
                match &response {
                    SpawnWorkflowResponse::ResponseError(err) => Err(anyhow!("{}", err)),
                    SpawnWorkflowResponse::WorkflowInstance(val) => Ok(val.workflow_id.workflow_id.to_string()),
                }
            }
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
    if args.wf_type.to_lowercase() == "help" {
        for wf_type in WorkflowType::examples() {
            println!("{}", wf_type);
        }
        return Ok(());
    }
    let wf_type = match WorkflowType::new(&args.wf_type) {
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
        "single" => ArrivalModel::Single,
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

    // Start the Redis dumper
    let mut redis_client =
        edgeless_benchmark::redis_dumper::RedisDumper::new(&args.redis_url, additional_fields.join(","), additional_header.join(","));
    let mut redis_client = match redis_client {
        Ok(val) => Some(val),
        Err(err) => {
            log::error!("could not connect to Redis at {}: {}", &args.redis_url, err);
            None
        }
    };

    // Create an e-ORC client
    let mut client_interface = ClientInterface::new(&args.controller_url, wf_type, args.seed + 1000, redis_client).await;

    // event queue, the first event is always a new workflow arriving at time 0
    let mut events = BinaryHeap::new();
    events.push(Event::WfNew(0_u64)); // in us

    // add the end-of-experiment event
    events.push(Event::WfExperimentEnd(to_microseconds(args.duration)));

    // set up warm-up period configuration
    if args.warmup >= args.duration {
        log::warn!(
            "metrics will not be collected since warm-up period ({} s) >= experiment duration ({} s)",
            args.warmup,
            args.duration
        );
    }
    let single_trigger_workflow_id = match setup_metrics_collector(&mut client_interface, &args.single_trigger_wasm, args.warmup).await {
        Ok(workflow_id) => workflow_id,
        Err(err) => anyhow::bail!("error when setting up the metrics collector: {} ", err),
    };

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
                    if let ArrivalModel::IncrAndKeep = arrival_model {
                        if now >= to_microseconds(args.warmup) {
                            continue;
                        }
                    }

                    wf_requested += 1;
                    match client_interface.start_workflow().await {
                        Ok(uuid) => {
                            wf_started += 1;
                            let end_time = match arrival_model {
                                ArrivalModel::Poisson => now + to_microseconds(lifetime_rv.sample(&mut rng)),
                                _ => to_microseconds(args.duration) - 1,
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
                            ArrivalModel::Single => args.duration + 1.0,
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
    let _ = client_interface.stop_workflow(&single_trigger_workflow_id).await;

    // dump data collected in Redis
    client_interface.dump(&args.output, args.append);

    // output metrics
    let blocking_probability = 1.0 - wf_started as f64 / wf_requested as f64;

    log::info!("workflow requested   = {}", wf_requested);
    log::info!("workflow started     = {}", wf_started);
    log::info!("blocking probability = {}", blocking_probability);

    Ok(())
}
