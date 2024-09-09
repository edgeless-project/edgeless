// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use anyhow::anyhow;

pub enum WorkflowType {
    None,

    // A single function.
    // 0: function.json path
    // 1: function.wasm path
    Single(String, String),

    // A chain of functions, each performing the multiplication of two matrices
    // of 32-bit floating point random numbers at each invocation.
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

    // A chain of functions, each performing the multiplication of an internal
    // random matrix of 32-bit floating point numbers by the input vector
    // received from the caller.
    // 0: min chain length
    // 1: max chain length
    // 2: min input size
    // 3: max input size
    // 4: vector_mul.wasm path
    VectorMulChain(u32, u32, u32, u32, String),

    // A workflow consisting of a random number of stages, where each stage
    // is composed of a random number of processing blocks. Before going to the
    // next stage, the output from all the processing blocks in the stage before
    // must be received.
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
    pub fn new(wf_type: &str) -> anyhow::Result<Self> {
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

    pub fn check(self) -> anyhow::Result<Self> {
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

    pub fn metrics_collector(&self) -> bool {
        match self {
            WorkflowType::None | WorkflowType::Single(_, _) => false,
            _ => true,
        }
    }

    pub fn examples() -> Vec<Self> {
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
