// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::workflow_type::WorkflowType;
use anyhow::anyhow;
use edgeless_api::api::controller::ControllerAPI;
use edgeless_api::workflow_instance::{SpawnWorkflowResponse, WorkflowFunction, WorkflowId, WorkflowInstanceAPI};
use rand::prelude::*;
use rand::SeedableRng;
use std::str::FromStr;

const ALPHA: f64 = 0.9_f64;

fn timestamp_now() -> String {
    let duration = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    format!("{}.{}", duration.as_secs(), duration.subsec_millis())
}

pub async fn setup_metrics_collector(engine: &mut Engine, single_trigger_wasm: &str, warmup: f64) -> anyhow::Result<String> {
    let function_class_code = match std::fs::read(single_trigger_wasm) {
        Ok(code) => code,
        Err(err) => anyhow::bail!("cannot read source: {}", err),
    };
    let res = engine
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

/// Engine for the creation/termination of EDGELESS workflows.
pub struct Engine {
    /// The client interface.
    pub client: Box<dyn WorkflowInstanceAPI>,
    /// Type of workflows generated.
    wf_type: WorkflowType,
    /// Pseudo-random number generator.
    rng: rand::rngs::StdRng,
    /// Identifier of the next workflow to start.
    wf_id: u32,
    /// Redis client.
    redis_client: Option<crate::redis_dumper::RedisDumper>,
    /// Mapping between UUID and name of the admitted workflows.
    uuid_to_names: std::collections::HashMap<uuid::Uuid, String>,
}

impl Engine {
    pub async fn new(controller_url: &str, wf_type: WorkflowType, seed: u64, redis_client: Option<crate::redis_dumper::RedisDumper>) -> Self {
        Self {
            client: edgeless_api::grpc_impl::controller::ControllerAPIClient::new(controller_url)
                .await
                .workflow_instance_api(),
            wf_type,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
            wf_id: 0,
            redis_client,
            uuid_to_names: std::collections::HashMap::new(),
        }
    }

    ///
    /// Dump the content of the Redis file to a set of output CSV files.
    ///
    /// - `dataset_path`: the path where to save the files.
    /// - `append`: if true append to the output files, if they exist already.
    ///
    pub fn dump(&mut self, dataset_path: &str, append: bool) {
        if let Some(redis_client) = &mut self.redis_client {
            if let Err(err) = redis_client.dump_csv(dataset_path, append) {
                log::error!("error dumping from Redis: {}", err);
            }
        }
    }

    pub async fn start_workflow(&mut self) -> anyhow::Result<String> {
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
            WorkflowType::MatrixMulChain(data) => {
                let chain_size: u32 = draw(data.min_chain_length, data.max_chain_length);

                let mut matrix_sizes = vec![];

                for i in 0..chain_size {
                    let mut outputs = vec!["metric".to_string()];
                    for k in 0..20 {
                        outputs.push(format!("out-{}", k).to_string());
                    }
                    let mut output_mapping = std::collections::HashMap::from([("metric".to_string(), "metrics-collector".to_string())]);
                    if i != (chain_size - 1) {
                        output_mapping.insert("out-0".to_string(), format!("f{}", (i + 1)));
                    } else if data.transaction_interval == 0 {
                        assert!(i == (chain_size - 1));
                        output_mapping.insert("out-0".to_string(), "f0".to_string());
                    }
                    let matrix_size = draw(data.min_matrix_size, data.max_matrix_size);
                    matrix_sizes.push(matrix_size);

                    let name = format!("f{}", i);
                    let annotations = std::collections::HashMap::from([(
                        "init-payload".to_string(),
                        format!(
                            "seed={},inter_arrival={},is_first={},is_last={},matrix_size={},outputs={}",
                            i,
                            data.transaction_interval,
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
                            function_class_code: std::fs::read(&data.function_wasm_path).unwrap(),
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
            WorkflowType::VectorMulChain(data) => {
                let chain_size = draw(data.max_chain_length, data.max_chain_length);
                let input_size = draw(data.min_input_size, data.max_input_size);

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
                            function_class_code: std::fs::read(&data.function_wasm_path).unwrap(),
                            function_class_outputs: vec!["metric".to_string(), "out".to_string()],
                        },
                        output_mapping,
                        annotations,
                    });
                }

                log::info!("wf{}, chain size {}, input size {}", self.wf_id, chain_size, input_size);
            }
            WorkflowType::MapReduce(data) => {
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
                let interval = draw(data.min_transaction_interval_ms, data.max_transaction_interval_ms);
                let size = draw(data.min_input_size, data.max_input_size);
                let stages = draw(data.min_num_stages, data.max_num_stages);

                let path = std::path::Path::new(&data.functions_path);

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
                    let breadth = draw(data.min_fan_out, data.max_fan_out);
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
                        let fibonacci = draw(data.min_fibonacci, data.max_fibonacci);
                        fibonacci_values.last_mut().unwrap().push(fibonacci);
                        let allocate = draw(data.min_memory_bytes, data.max_memory_bytes);
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
            redis_client.set(format!("workflow:{}:begin", wf_name).as_str(), &timestamp_now());
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
                    SpawnWorkflowResponse::WorkflowInstance(val) => {
                        self.uuid_to_names.insert(val.workflow_id.workflow_id, wf_name.clone());
                        Ok(val.workflow_id.workflow_id.to_string())
                    }
                }
            }
            Err(err) => {
                panic!("error when stopping a workflow: {}", err);
            }
        }
    }

    pub async fn stop_workflow(&mut self, uuid: &str) -> anyhow::Result<()> {
        if let Some(redis_client) = &mut self.redis_client {
            if let Some(wf_name) = self.uuid_to_names.get(&uuid::Uuid::from_str(uuid).unwrap()) {
                redis_client.set(format!("workflow:{}:end", wf_name).as_str(), &timestamp_now());
            }
        }
        let res = self.client.stop(WorkflowId::from_string(uuid)).await;
        match res {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }
}
