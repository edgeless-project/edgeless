pub use edgeless_api_core::instance_id::*;

use crate::common::PatchRequest;

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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct FunctionClassSpecification {
    pub function_class_id: String,
    pub function_class_type: String,
    pub function_class_version: String,
    pub function_class_inlude_code: Vec<u8>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnFunctionRequest {
    pub instance_id: Option<InstanceId>,
    pub code: FunctionClassSpecification,
    pub annotations: std::collections::HashMap<String, String>,
    pub state_specification: StateSpecification,
}

#[async_trait::async_trait]
pub trait FunctionInstanceOrcAPI: FunctionInstanceOrcAPIClone + Sync + Send {
    async fn start_function(
        &mut self,
        spawn_request: SpawnFunctionRequest,
    ) -> anyhow::Result<crate::common::StartComponentResponse<super::orc::DomainManagedInstanceId>>;
    async fn stop_function(&mut self, id: super::orc::DomainManagedInstanceId) -> anyhow::Result<()>;
    async fn patch(&mut self, update: PatchRequest) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait FunctionInstanceNodeAPI: FunctionInstanceNodeAPIClone + Sync + Send {
    async fn start(
        &mut self,
        spawn_request: SpawnFunctionRequest,
    ) -> anyhow::Result<crate::common::StartComponentResponse<edgeless_api_core::instance_id::InstanceId>>;
    async fn stop(&mut self, id: InstanceId) -> anyhow::Result<()>;
    async fn patch(&mut self, update: PatchRequest) -> anyhow::Result<()>;
}

// https://stackoverflow.com/a/30353928
pub trait FunctionInstanceOrcAPIClone {
    fn clone_box(&self) -> Box<dyn FunctionInstanceOrcAPI>;
}
impl<T> FunctionInstanceOrcAPIClone for T
where
    T: 'static + FunctionInstanceOrcAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn FunctionInstanceOrcAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn FunctionInstanceOrcAPI> {
    fn clone(&self) -> Box<dyn FunctionInstanceOrcAPI> {
        self.clone_box()
    }
}

pub trait FunctionInstanceNodeAPIClone {
    fn clone_box(&self) -> Box<dyn FunctionInstanceNodeAPI>;
}
impl<T> FunctionInstanceNodeAPIClone for T
where
    T: 'static + FunctionInstanceNodeAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn FunctionInstanceNodeAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn FunctionInstanceNodeAPI> {
    fn clone(&self) -> Box<dyn FunctionInstanceNodeAPI> {
        self.clone_box()
    }
}
