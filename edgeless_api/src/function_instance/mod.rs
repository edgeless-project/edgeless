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
    pub annotations: std::collections::HashMap<String, String>,
    pub state_specification: StateSpecification,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StartResourceRequest {
    pub class_type: String,
    pub configurations: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceProviderSpecification {
    pub provider_id: String,
    pub class_type: String,
    pub outputs: Vec<String>,
    pub configuration_url: String,
}

impl std::fmt::Display for ResourceProviderSpecification {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "provider_id {}, class_type {}, outputs [{}], configuration_url {}",
            self.provider_id,
            self.class_type,
            self.outputs.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","),
            self.configuration_url
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateNodeRequest {
    // 0: node_id (cannot be nil)
    // 1: agent_url (cannot be empty)
    // 2: invocation_url (cannot be empty)
    // 3: resource provider specifications (can be empty)
    Registration(uuid::Uuid, String, String, Vec<ResourceProviderSpecification>),

    // 0: node_id (cannot be empty)
    Deregistration(uuid::Uuid),
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

#[derive(Debug, Clone, PartialEq)]
pub struct PatchRequest {
    pub instance_id: Option<InstanceId>,
    pub output_mapping: std::collections::HashMap<String, InstanceId>,
}

#[async_trait::async_trait]
pub trait FunctionInstanceOrcAPI: FunctionInstanceOrcAPIClone + Sync + Send {
    async fn start_function(&mut self, spawn_request: SpawnFunctionRequest) -> anyhow::Result<crate::common::StartComponentResponse>;
    async fn stop_function(&mut self, id: InstanceId) -> anyhow::Result<()>;
    async fn start_resource(&mut self, spawn_request: StartResourceRequest) -> anyhow::Result<crate::common::StartComponentResponse>;
    async fn stop_resource(&mut self, id: InstanceId) -> anyhow::Result<()>;
    async fn patch(&mut self, update: PatchRequest) -> anyhow::Result<()>;
    async fn update_node(&mut self, request: UpdateNodeRequest) -> anyhow::Result<UpdateNodeResponse>;
}

#[async_trait::async_trait]
pub trait FunctionInstanceNodeAPI: FunctionInstanceNodeAPIClone + Sync + Send {
    async fn start(&mut self, spawn_request: SpawnFunctionRequest) -> anyhow::Result<crate::common::StartComponentResponse>;
    async fn stop(&mut self, id: InstanceId) -> anyhow::Result<()>;
    async fn patch(&mut self, update: PatchRequest) -> anyhow::Result<()>;
    async fn update_peers(&mut self, request: UpdatePeersRequest) -> anyhow::Result<()>;
    async fn keep_alive(&mut self) -> anyhow::Result<()>;
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
