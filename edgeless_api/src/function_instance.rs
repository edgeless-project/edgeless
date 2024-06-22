// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub use edgeless_api_core::instance_id::*;

#[derive(Debug, Clone, PartialEq)]
pub enum StatePolicy {
    Transient,
    NodeLocal,
    Global,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateSpecification {
    pub state_id: uuid::Uuid,
    pub state_policy: StatePolicy,
}

impl Default for StateSpecification {
    fn default() -> Self {
        Self {
            state_id: uuid::Uuid::nil(),
            state_policy: StatePolicy::NodeLocal,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct FunctionClassSpecification {
    /// Function class identifier.
    pub function_class_id: String,
    /// Run-time agent type this function is made for.
    pub function_class_type: String,
    /// Function class version.
    pub function_class_version: String,
    /// Inline function's code (if present).
    pub function_class_code: Vec<u8>,
    /// Output channels in which the function may generate new. Can be empty.
    pub function_class_outputs: Vec<String>,
}

impl Default for FunctionClassSpecification {
    fn default() -> Self {
        Self {
            function_class_id: "".to_string(),
            function_class_type: "".to_string(),
            function_class_version: "".to_string(),
            function_class_code: vec![],
            function_class_outputs: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SpawnFunctionRequest {
    #[serde(skip)]
    pub instance_id: Option<InstanceId>,
    #[serde(skip)]
    pub code: FunctionClassSpecification,
    pub annotations: std::collections::HashMap<String, String>,
    #[serde(skip)]
    pub state_specification: StateSpecification,
}

#[async_trait::async_trait]
pub trait FunctionInstanceAPI<FunctionIdType: Clone>: FunctionInstanceAPIClone<FunctionIdType> + Sync + Send {
    async fn start(&mut self, spawn_request: SpawnFunctionRequest) -> anyhow::Result<crate::common::StartComponentResponse<FunctionIdType>>;
    async fn stop(&mut self, id: FunctionIdType) -> anyhow::Result<()>;
    async fn patch(&mut self, update: crate::common::PatchRequest) -> anyhow::Result<()>;
}

// https://stackoverflow.com/a/30353928
pub trait FunctionInstanceAPIClone<FunctionIdType: Clone> {
    fn clone_box(&self) -> Box<dyn FunctionInstanceAPI<FunctionIdType>>;
}
impl<T, FunctionIdType: Clone> FunctionInstanceAPIClone<FunctionIdType> for T
where
    T: 'static + FunctionInstanceAPI<FunctionIdType> + Clone,
{
    fn clone_box(&self) -> Box<dyn FunctionInstanceAPI<FunctionIdType>> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn FunctionInstanceAPI<crate::function_instance::InstanceId>> {
    fn clone(&self) -> Box<dyn FunctionInstanceAPI<crate::function_instance::InstanceId>> {
        self.clone_box()
    }
}

impl Clone for Box<dyn FunctionInstanceAPI<crate::orc::DomainManagedInstanceId>> {
    fn clone(&self) -> Box<dyn FunctionInstanceAPI<crate::orc::DomainManagedInstanceId>> {
        self.clone_box()
    }
}
