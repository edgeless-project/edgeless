// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use serde::{Deserialize};

#[derive(Debug, serde::Deserialize)]
pub struct WorkflowSpecFunctionClass {
    pub id: String,
    pub function_type: String,
    pub version: String,
    pub code: Option<String>,
    pub build: Option<String>,
    pub outputs: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct WorflowSpecFunction {
    pub name: String,
    pub class_specification: WorkflowSpecFunctionClass,
    pub output_mapping: std::collections::HashMap<String, String>,
    pub annotations: std::collections::HashMap<String, String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct WorkflowSpecResourceInstance {
    pub name: String,
    pub class_type: String,
    pub output_mapping: std::collections::HashMap<String, String>,
    pub configurations: std::collections::HashMap<String, String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct WorkflowSpec {
    pub functions: Vec<WorflowSpecFunction>,
    pub resources: Vec<WorkflowSpecResourceInstance>,
    pub annotations: std::collections::HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct RepoEndpoint {
    pub url: Url,
    pub credential: Credential,
}

// Config struct holds to data from the `[config]` section.
#[derive(Deserialize)]
pub struct Url {
    pub name: String,
}

#[derive(Deserialize)]
pub struct Credential {
    pub basic_auth_user: String,
    pub basic_auth_pass: String,
}