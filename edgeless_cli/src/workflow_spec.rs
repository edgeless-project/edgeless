// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

// #[derive(Debug, serde::Deserialize)]
// pub struct WorkflowSpecFunctionClass {
//     pub id: String,
//     pub function_type: String,
//     pub version: String,
//     pub code: Option<String>,
//     pub build: Option<String>,
//     pub outputs: std::collections::HashMap<String, PortDefinition>,
//     pub inputs: std::collections::HashMap<String, PortDefinition>,
//     pub inner_structure: Vec<Mapping>,
// }
// #[derive(Debug, serde::Deserialize, PartialEq)]
// pub struct Mapping {
//     pub source: MappingNode,
//     pub dests: Vec<MappingNode>,
// }

// #[derive(Debug, serde::Deserialize, PartialEq)]
// #[serde(tag = "type", content = "port_id")]
// pub enum MappingNode {
//     PORT(String),
//     SIDE_EFFECT,
// }

// #[derive(Debug, serde::Deserialize)]
// pub struct DirectTarget {
//     pub target_component: String,
//     pub port: String,
// }

// #[derive(Debug, serde::Deserialize)]
// pub struct TopicTarget {
//     pub topic: String,
// }

// #[derive(Debug, serde::Deserialize)]
// #[serde(tag = "type", content = "config")]
// pub enum PortMapping {
//     DIRECT(DirectTarget),
//     ANY_OF(Vec<DirectTarget>),
//     ALL_OF(Vec<DirectTarget>),
//     TOPIC(TopicTarget),
// }

// #[derive(Debug, serde::Deserialize)]
// pub enum PortMethod {
//     CAST,
//     CALL,
// }

// #[derive(Debug, serde::Deserialize)]
// pub struct PortDefinition {
//     pub method: PortMethod,
//     pub data_type: String,
//     pub return_data_type: Option<String>,
// }

// #[derive(Debug, serde::Deserialize)]
// pub struct WorflowSpecFunction {
//     pub name: String,
//     pub class_specification: WorkflowSpecFunctionClass,
//     pub output_mapping: std::collections::HashMap<String, PortMapping>,
//     pub input_mapping: std::collections::HashMap<String, PortMapping>,
//     pub annotations: std::collections::HashMap<String, String>,
// }

// #[derive(Debug, serde::Deserialize)]
// pub struct WorkflowSpecResourceInstance {
//     pub name: String,
//     pub class_type: String,
//     pub output_mapping: std::collections::HashMap<String, PortMapping>,
//     pub input_mapping: std::collections::HashMap<String, PortMapping>,
//     pub configurations: std::collections::HashMap<String, String>,
// }

// #[derive(Debug, serde::Deserialize)]
// pub struct WorkflowSpec {
//     pub functions: Vec<WorflowSpecFunction>,
//     pub resources: Vec<WorkflowSpecResourceInstance>,
//     pub annotations: std::collections::HashMap<String, String>,
// }
