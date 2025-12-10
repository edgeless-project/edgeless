// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub use edgeless_api_core::event_metadata::*;
pub use edgeless_api_core::event_timestamp::*;
pub use edgeless_api_core::instance_id::*;

include!("function_instance_structs.rs");

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StatePolicy {
    Transient,
    NodeLocal,
    Global,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

impl FunctionClassSpecification {
    pub fn to_short_string(&self) -> String {
        format!(
            "run-time {} class {} version {} code {:?}",
            self.function_type, self.id, self.version, self.code
        )
    }

    /// Return a version of the object with the binary stripped.
    pub fn strip(&self) -> Self {
        Self {
            id: self.id.clone(),
            function_type: self.function_type.clone(),
            version: self.version.clone(),
            binary: None,
            code: self.code.clone(),
            outputs: self.outputs.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SpawnFunctionRequest {
    pub spec: FunctionClassSpecification,
    pub annotations: std::collections::HashMap<String, String>,
    pub state_specification: StateSpecification,
    pub workflow_id: String,
    pub replication_factor: Option<u32>,
}

impl SpawnFunctionRequest {
    /// Remove the function_class_code from the return value if RUST_WASM.
    pub fn strip(&self) -> Self {
        Self {
            spec: self.spec.strip(),
            annotations: self.annotations.clone(),
            state_specification: self.state_specification.clone(),
            workflow_id: self.workflow_id.clone(),
            replication_factor: self.replication_factor,
        }
    }
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

impl Clone for Box<dyn FunctionInstanceAPI<crate::function_instance::DomainManagedInstanceId>> {
    fn clone(&self) -> Box<dyn FunctionInstanceAPI<crate::function_instance::DomainManagedInstanceId>> {
        self.clone_box()
    }
}
