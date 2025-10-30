// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Default, schemars::JsonSchema)]
pub struct FunctionClassSpecification {
    /// ID / Class name of the function
    pub id: String,
    /// Run-time agent type this function is made for.
    /// One of: RUST_WASM or CONTAINER;
    pub function_type: String,
    /// Semantic versioning; defined by the function developer
    pub version: String,
    /// WASM: inline function's code.
    /// CONTAINER: None.
    pub binary: Option<Vec<u8>>,
    /// (optional) Only relevant in a workflow.
    /// WASM: path to an object file with the binary of the function's code.
    /// CONTAINER: the image:tag
    pub code: Option<String>,
    /// Defines the outputs of this function; these outputs can be then mapped
    /// in the workflow. Should not contain duplicated outputs. Can be empty.
    pub outputs: Vec<String>,
}
