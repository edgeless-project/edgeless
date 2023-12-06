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
    pub output_mapping: std::collections::HashMap<String, InstanceId>,
    pub annotations: std::collections::HashMap<String, String>,
    pub state_specification: StateSpecification,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpawnFunctionResponse {
    ResponseError(crate::common::ResponseError),
    InstanceId(InstanceId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateNodeRequest {
    Registration(uuid::Uuid, String, String), // node_id, agent_url, invocation_url
    Deregistration(uuid::Uuid),               // node_id
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateNodeResponse {
    ResponseError(crate::common::ResponseError),
    Accepted,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdatePeersRequest {
    Add(uuid::Uuid, String), // node_id, invocation_url
    Del(uuid::Uuid),         // node_id
    Clear,
}

#[derive(Debug, Clone)]
pub struct UpdateFunctionLinksRequest {
    pub instance_id: Option<InstanceId>,
    pub output_mapping: std::collections::HashMap<String, InstanceId>,
}

#[async_trait::async_trait]
pub trait FunctionInstanceAPI: FunctionInstanceAPIClone + Sync + Send {
    async fn start(&mut self, spawn_request: SpawnFunctionRequest) -> anyhow::Result<SpawnFunctionResponse>;
    async fn stop(&mut self, id: InstanceId) -> anyhow::Result<()>;
    async fn update_links(&mut self, update: UpdateFunctionLinksRequest) -> anyhow::Result<()>;
    async fn update_node(&mut self, request: UpdateNodeRequest) -> anyhow::Result<UpdateNodeResponse>;
    async fn update_peers(&mut self, request: UpdatePeersRequest) -> anyhow::Result<()>;
    async fn keep_alive(&mut self) -> anyhow::Result<()>;
}

// https://stackoverflow.com/a/30353928
pub trait FunctionInstanceAPIClone {
    fn clone_box(&self) -> Box<dyn FunctionInstanceAPI>;
}
impl<T> FunctionInstanceAPIClone for T
where
    T: 'static + FunctionInstanceAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn FunctionInstanceAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn FunctionInstanceAPI> {
    fn clone(&self) -> Box<dyn FunctionInstanceAPI> {
        self.clone_box()
    }
}
