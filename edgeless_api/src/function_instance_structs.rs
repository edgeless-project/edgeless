// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Clone, serde::Deserialize, serde::Serialize, PartialEq, Default, schemars::JsonSchema)]
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

impl std::fmt::Debug for FunctionClassSpecification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let binary_info = match &self.binary {
            Some(binary) => format!("Some(<{} bytes>)", binary.len()),
            None => "None".to_string(),
        };
        
        f.debug_struct("FunctionClassSpecification")
            .field("id", &self.id)
            .field("function_type", &self.function_type)
            .field("version", &self.version)
            .field("binary", &binary_info)
            .field("code", &self.code)
            .field("outputs", &self.outputs)
            .finish()
    }
}
