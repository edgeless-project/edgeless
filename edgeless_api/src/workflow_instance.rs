// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use std::str::FromStr;

const WORKFLOW_ID_NONE: uuid::Uuid = uuid::uuid!("00000000-0000-0000-0000-ffff00000000");

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkflowId {
    pub workflow_id: uuid::Uuid,
}

impl WorkflowId {
    pub fn from_string(s: &str) -> Self {
        Self {
            workflow_id: uuid::Uuid::from_str(s).unwrap(),
        }
    }
    pub fn to_string(&self) -> String {
        self.workflow_id.to_string()
    }
    pub fn none() -> Self {
        Self {
            workflow_id: WORKFLOW_ID_NONE,
        }
    }
    pub fn is_valid(&self) -> Option<&WorkflowId> {
        if self.workflow_id == WORKFLOW_ID_NONE {
            None
        } else {
            Some(self)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowFunctionMapping {
    pub name: String,
    pub function_id: crate::function_instance::ComponentId,
    pub domain_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowInstance {
    pub workflow_id: WorkflowId,
    pub domain_mapping: Vec<WorkflowFunctionMapping>,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct WorkflowResource {
    pub name: String,
    pub class_type: String,
    pub output_mapping: std::collections::HashMap<String, String>,
    pub configurations: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct WorkflowFunction {
    pub name: String,
    pub function_class_specification: crate::function_instance::FunctionClassSpecification,
    pub output_mapping: std::collections::HashMap<String, String>,
    pub annotations: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpawnWorkflowRequest {
    pub workflow_functions: Vec<WorkflowFunction>,
    pub workflow_resources: Vec<WorkflowResource>,
    pub annotations: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpawnWorkflowResponse {
    ResponseError(crate::common::ResponseError),
    WorkflowInstance(WorkflowInstance),
}

#[async_trait::async_trait]
pub trait WorkflowInstanceAPI: WorkflowInstanceAPIClone + Send + Sync {
    async fn start(&mut self, request: SpawnWorkflowRequest) -> anyhow::Result<SpawnWorkflowResponse>;
    async fn stop(&mut self, id: WorkflowId) -> anyhow::Result<()>;
    async fn list(&mut self, id: WorkflowId) -> anyhow::Result<Vec<WorkflowInstance>>;
}

// https://stackoverflow.com/a/30353928
pub trait WorkflowInstanceAPIClone {
    fn clone_box(&self) -> Box<dyn WorkflowInstanceAPI>;
}
impl<T> WorkflowInstanceAPIClone for T
where
    T: 'static + WorkflowInstanceAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn WorkflowInstanceAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn WorkflowInstanceAPI> {
    fn clone(&self) -> Box<dyn WorkflowInstanceAPI> {
        self.clone_box()
    }
}
