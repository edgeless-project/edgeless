// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use std::io::Read;

use anyhow::anyhow;

#[derive(Debug, serde::Deserialize, serde::Serialize)]

pub struct MatrixMulChainData {
    pub min_chain_length: u32,
    pub max_chain_length: u32,
    pub min_matrix_size: u32,
    pub max_matrix_size: u32,
    // interval between consecutive transactions, in ms
    // if 0 then make the workflow circular, i.e., the last
    // function calls the first one to trigger a new
    // transaction
    pub transaction_interval: u32,
    pub function_wasm_path: String,
}

impl Default for MatrixMulChainData {
    fn default() -> Self {
        Self {
            min_chain_length: 1,
            max_chain_length: 3,
            min_matrix_size: 100,
            max_matrix_size: 500,
            transaction_interval: 0,
            function_wasm_path: "functions/matrix_mul/matrix_mul.wasm".to_string(),
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VectorMulChainData {
    pub min_chain_length: u32,
    pub max_chain_length: u32,
    pub min_input_size: u32,
    pub max_input_size: u32,
    pub function_wasm_path: String,
}

impl Default for VectorMulChainData {
    fn default() -> Self {
        Self {
            min_chain_length: 1,
            max_chain_length: 3,
            min_input_size: 100,
            max_input_size: 500,
            function_wasm_path: "functions/vector_mul/vector_mul.wasm".to_string(),
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]

pub struct MapReduceData {
    pub min_transaction_interval_ms: u32,
    pub max_transaction_interval_ms: u32,
    pub min_input_size: u32,
    pub max_input_size: u32,
    pub min_num_stages: u32,
    pub max_num_stages: u32,
    pub min_fan_out: u32,
    pub max_fan_out: u32,
    pub min_fibonacci: u32,
    pub max_fibonacci: u32,
    pub min_memory_bytes: u32,
    pub max_memory_bytes: u32,
    pub functions_path: String,
}

impl Default for MapReduceData {
    fn default() -> Self {
        Self {
            min_transaction_interval_ms: 500,
            max_transaction_interval_ms: 1500,
            min_input_size: 100,
            max_input_size: 1000,
            min_num_stages: 1,
            max_num_stages: 3,
            min_fan_out: 1,
            max_fan_out: 3,
            min_fibonacci: 1000,
            max_fibonacci: 5000,
            min_memory_bytes: 0,
            max_memory_bytes: 0,
            functions_path: "functions/".to_string(),
        }
    }
}

pub struct JsonSpecData {
    pub spec_string: String,
    pub parent_path: Box<std::path::Path>,
}

pub enum WorkflowType {
    None,

    // A single function.
    // 0: function.json path
    // 1: function.wasm path
    Single(String, String),

    // A chain of functions, each performing the multiplication of two matrices
    // of 32-bit floating point random numbers at each invocation.
    MatrixMulChain(MatrixMulChainData),

    // A chain of functions, each performing the multiplication of an internal
    // random matrix of 32-bit floating point numbers by the input vector
    // received from the caller.
    VectorMulChain(VectorMulChainData),

    // A workflow consisting of a random number of stages, where each stage
    // is composed of a random number of processing blocks. Before going to the
    // next stage, the output from all the processing blocks in the stage before
    // must be received.
    MapReduce(MapReduceData),

    // A workflow provided in a JSON spec file in the path given.
    // The string @WFID is substituted with the workflow counter.
    JsonSpec(JsonSpecData),
}

impl WorkflowType {
    pub fn new(wf_type: &str) -> anyhow::Result<Self> {
        let tokens: Vec<&str> = wf_type.split(';').collect();
        if !tokens.is_empty() && tokens[0] == "none" {
            return WorkflowType::None.check();
        } else if !tokens.is_empty() && tokens[0] == "single" && tokens.len() == 3 {
            return WorkflowType::Single(tokens[1].to_string(), tokens[2].to_string()).check();
        } else if !tokens.is_empty() && tokens[0] == "matrix-mul-chain" && tokens.len() == 2 {
            if tokens[1] == "template" {
                println!("{}\n", serde_json::to_string_pretty(&MatrixMulChainData::default()).unwrap());
                anyhow::bail!("enjoy your template file, which you can save by redirecting stdout to file");
            }
            let file = std::fs::File::open(tokens[1])?;
            let reader = std::io::BufReader::new(file);
            let data: MatrixMulChainData = serde_json::from_reader(reader)?;
            return WorkflowType::MatrixMulChain(data).check();
        } else if !tokens.is_empty() && tokens[0] == "vector-mul-chain" && tokens.len() == 2 {
            if tokens[1] == "template" {
                println!("{}\n", serde_json::to_string_pretty(&VectorMulChainData::default()).unwrap());
                anyhow::bail!("enjoy your template file, which you can save by redirecting stdout to file");
            }
            let file = std::fs::File::open(tokens[1])?;
            let reader = std::io::BufReader::new(file);
            let data: VectorMulChainData = serde_json::from_reader(reader)?;
            return WorkflowType::VectorMulChain(data).check();
        } else if !tokens.is_empty() && tokens[0] == "map-reduce" && tokens.len() == 2 {
            if tokens[1] == "template" {
                println!("{}\n", serde_json::to_string_pretty(&MapReduceData::default()).unwrap());
                anyhow::bail!("enjoy your template file, which you can save by redirecting stdout to file");
            }
            let file = std::fs::File::open(tokens[1])?;
            let reader = std::io::BufReader::new(file);
            let data: MapReduceData = serde_json::from_reader(reader)?;
            return WorkflowType::MapReduce(data).check();
        } else if !tokens.is_empty() && tokens[0] == "json-spec" && tokens.len() == 2 {
            let file = std::fs::File::open(tokens[1])?;
            let mut reader = std::io::BufReader::new(file);
            let mut spec_string = String::default();
            reader.read_to_string(&mut spec_string)?;
            let parent_path = std::path::Path::new(tokens[1])
                .parent()
                .expect("cannot find the workflow spec's parent path")
                .into();
            return WorkflowType::JsonSpec(JsonSpecData { spec_string, parent_path }).check();
        }
        Err(anyhow!("unknown workflow type: {}", wf_type))
    }

    pub fn check(self) -> anyhow::Result<Self> {
        match &self {
            WorkflowType::None => {}
            WorkflowType::Single(json, wasm) => {
                anyhow::ensure!(!json.is_empty(), "empty JSON file path");
                anyhow::ensure!(!wasm.is_empty(), "empty WASM file path");
            }
            WorkflowType::VectorMulChain(data) => {
                anyhow::ensure!(data.min_chain_length > 0, "vanishing min chain");
                anyhow::ensure!(data.max_chain_length >= data.min_chain_length, "chain: min > max");
                anyhow::ensure!(data.max_input_size >= data.min_input_size, "size: min > max");
                anyhow::ensure!(!data.function_wasm_path.is_empty(), "empty WASM file path");
            }
            WorkflowType::MatrixMulChain(data) => {
                anyhow::ensure!(data.min_chain_length > 0, "vanishing min chain");
                anyhow::ensure!(data.max_chain_length >= data.min_chain_length, "chain: min > max");
                anyhow::ensure!(data.max_matrix_size >= data.min_matrix_size, "size: min > max");
                anyhow::ensure!(!data.function_wasm_path.is_empty(), "empty WASM file path");
            }
            WorkflowType::MapReduce(data) => {
                anyhow::ensure!(data.min_transaction_interval_ms > 0, "vanishing min interval");
                anyhow::ensure!(
                    data.max_transaction_interval_ms >= data.min_transaction_interval_ms,
                    "interval: min > max"
                );
                anyhow::ensure!(data.max_input_size >= data.min_input_size, "rate: min > max");
                anyhow::ensure!(data.min_num_stages > 0, "vanishing min stages");
                anyhow::ensure!(data.max_num_stages >= data.min_num_stages, "rate: min > max");
                anyhow::ensure!(data.min_fan_out > 0, "vanishing fan-out");
                anyhow::ensure!(data.max_fan_out >= data.min_fan_out, "fan-out: min > max");
                anyhow::ensure!(data.max_fibonacci >= data.min_fibonacci, "fibonacci: min > max");
                anyhow::ensure!(data.max_memory_bytes >= data.min_memory_bytes, "allocation: min > max");
                anyhow::ensure!(!data.functions_path.is_empty(), "empty library path");
            }
            WorkflowType::JsonSpec(data) => {
                anyhow::ensure!(!data.spec_string.is_empty(), "empty workflow specification file");
            }
        }
        Ok(self)
    }

    pub fn all() -> [String; 6] {
        [
            "none".to_string(),
            "single".to_string(),
            "matrix-mul-chain (*)".to_string(),
            "vector-mul-chain (*)".to_string(),
            "map-reduce (*)".to_string(),
            "json-spec".to_string(),
        ]
    }

    pub fn metrics_collector(&self) -> bool {
        !matches!(self, WorkflowType::None | WorkflowType::Single(_, _))
    }
}
