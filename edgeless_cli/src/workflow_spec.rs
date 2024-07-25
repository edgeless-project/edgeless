// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of
// Connected Mobility SPDX-FileCopyrightText: © 2023 Claudio Cicconetti
// <c.cicconetti@iit.cnr.it> SPDX-License-Identifier: MIT

// NOTE: Serialize derived for all structs to silence the compiler because of
// JsonSchema

/// Defines the function class; required by the edgeless_cli to build the
/// function
#[derive(Debug, serde::Deserialize, schemars::JsonSchema, serde::Serialize)]
pub struct WorkflowSpecFunctionClass {
    /// ID / Class name of the function
    pub id: String,
    /// One of: RUST_WASM or CONTAINER;
    pub function_type: String,
    /// Semantic versioning; defined by the function developer
    pub version: String,
    /// (optional) only relevant in a workflow; For WASM: specifies the path to
    /// an object file that should be used as the function's code. For
    /// CONTAINER: specified the image:tag
    pub code: Option<String>,
    /// (optional) WARNING: this field is currently not used anywhere: TODO:
    /// deprecate; only relevant for edgeless_cli build system; identifies the
    /// entry point for the function build system; in case of Rust/WASM should
    /// be Cargo.toml
    #[allow(dead_code)]
    pub build: Option<String>,
    /// Defines the outputs of this function; these outputs can be then mapped
    /// in the workflow. Should not contain duplicated outputs
    pub outputs: Vec<String>,
}

/// Defines the function as a part of the workflow
#[derive(Debug, serde::Deserialize, schemars::JsonSchema, serde::Serialize)]
pub struct WorflowSpecFunction {
    /// Logical name of the function within this workflow. This name shall be
    /// used for mapping of outputs.
    pub name: String,
    /// specifies the class of the function using the function spec; NOTE:
    /// optional field code must be specified here!
    pub class_specification: WorkflowSpecFunctionClass,
    /// Maps the output of a function to the input of another function or
    /// resource. Uses the function / resource (logical) name as defined by the
    /// "name" property within the workflow spec.
    pub output_mapping: std::collections::HashMap<String, String>,
    /// Key-value pairs of annotations for the function
    pub annotations: std::collections::HashMap<String, String>,
}

/// Defines the resource as a part of the workflow
#[derive(Debug, serde::Deserialize, schemars::JsonSchema, serde::Serialize)]
pub struct WorkflowSpecResourceInstance {
    /// Logical name of the resource instance within this workflow. It should be
    // used to map outputs of functions to this resource inputs
    pub name: String,
    /// specifies the class of the resource used; Example resources:
    /// ["http-ingress", "http-egress", "file-log", "redis", "dda"]
    pub class_type: String,
    /// Maps the outputs of this resource to functions. Some resources may
    /// provide standard outputs that must be mapped - consult the documentation
    /// to find out more.
    pub output_mapping: std::collections::HashMap<String, String>,
    /// key-value configuration of the resource instance
    pub configurations: std::collections::HashMap<String, String>,
}

/// Defines the workflow to be deployed on edgeless framework
#[derive(Debug, serde::Deserialize, schemars::JsonSchema, serde::Serialize)]
pub struct WorkflowSpec {
    /// all functions that are used in this workflow
    pub functions: Vec<WorflowSpecFunction>,
    /// all resources that are used in this workflow
    pub resources: Vec<WorkflowSpecResourceInstance>,
    /// workflow specific annotations
    pub annotations: std::collections::HashMap<String, String>,
}
