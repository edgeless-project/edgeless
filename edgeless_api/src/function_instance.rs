// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub use edgeless_api_core::event_timestamp::*;
pub use edgeless_api_core::instance_id::*;

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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Default)]
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

impl FunctionClassSpecification {
    pub fn to_short_string(&self) -> String {
        format!(
            "run-time {} class {} version {}",
            self.function_class_type, self.function_class_id, self.function_class_version
        )
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SpawnFunctionRequest {
    pub code: FunctionClassSpecification,
    pub annotations: std::collections::HashMap<String, String>,
    pub state_specification: StateSpecification,
    pub workflow_id: String,
}

impl SpawnFunctionRequest {
    /// Remove the function_class_code from the return value if RUST_WASM.
    pub fn strip(&self) -> Self {
        let function_class_code = if self.code.function_class_type == "RUST_WASM" {
            vec![]
        } else {
            self.code.function_class_code.clone()
        };
        Self {
            code: FunctionClassSpecification {
                function_class_id: self.code.function_class_id.clone(),
                function_class_type: self.code.function_class_type.clone(),
                function_class_version: self.code.function_class_version.clone(),
                function_class_code,
                function_class_outputs: self.code.function_class_outputs.clone(),
            },
            annotations: self.annotations.clone(),
            state_specification: self.state_specification.clone(),
            workflow_id: self.workflow_id.clone(),
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
