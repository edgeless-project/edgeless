// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, schemars::JsonSchema)]
pub struct WorkflowFunction {
    /// Logical name of the function within this workflow. This name shall be
    /// used for mapping of outputs.
    pub name: String,
    /// Specifies the class of the function using the function spec; NOTE:
    /// optional field code must be specified here.
    pub class_specification: FunctionClassSpecification,
    /// Maps the output of a function to the input of another function or
    /// resource. Uses the function / resource (logical) name as defined by the
    /// "name" property within the workflow spec.
    pub output_mapping: std::collections::HashMap<String, String>,
    /// Key-value pairs of annotations for the function.
    pub annotations: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, schemars::JsonSchema)]
pub struct WorkflowResource {
    /// Logical name of the resource instance within this workflow.
    /// It should be used to map outputs of functions to this resource inputs.
    pub name: String,
    /// Specifies the class of the resource used; Example resources:
    /// ["http-ingress", "http-egress", "file-log", "redis", "dda"].
    pub class_type: String,
    /// Maps the outputs of this resource to functions. Some resources may
    /// provide standard outputs that must be mapped - consult the documentation
    /// to find out more.
    pub output_mapping: std::collections::HashMap<String, String>,
    /// Key-value configuration and annotations of the resource instance.
    pub configurations: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct SpawnWorkflowRequest {
    /// All the functions used in this workflow.
    pub functions: Vec<WorkflowFunction>,
    /// All the resources used in this workflow.
    pub resources: Vec<WorkflowResource>,
    /// Workflow specific annotations.
    pub annotations: std::collections::HashMap<String, String>,
}
