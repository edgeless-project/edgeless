#[derive(Debug, Clone)]
pub struct FunctionId {
    pub node_id: uuid::Uuid,
    pub function_id: uuid::Uuid,
}

impl FunctionId {
    pub fn new(node_id: uuid::Uuid) -> Self {
        Self {
            node_id: node_id,
            function_id: uuid::Uuid::new_v4(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum StatePolicy {
    Transient,
    NodeLocal,
    Global,
}

#[derive(Debug, Clone)]
pub struct StateSpecification {
    pub state_id: uuid::Uuid,
    pub state_policy: StatePolicy,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FunctionClassSpecification {
    pub function_class_id: String,
    pub function_class_type: String,
    pub function_class_version: String,
    pub function_class_inlude_code: Vec<u8>,
    pub output_callback_declarations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SpawnFunctionRequest {
    pub function_id: Option<FunctionId>,
    pub code: FunctionClassSpecification,
    pub output_callback_definitions: std::collections::HashMap<String, FunctionId>,
    pub return_continuation: FunctionId,
    pub annotations: std::collections::HashMap<String, String>,
    pub state_specification: StateSpecification,
}

#[derive(Debug)]
pub struct UpdateFunctionLinksRequest {
    pub function_id: Option<FunctionId>,
    pub output_callback_definitions: std::collections::HashMap<String, FunctionId>,
    pub return_continuation: FunctionId,
}

#[async_trait::async_trait]
pub trait FunctionInstanceAPI: FunctionInstanceAPIClone + Sync + Send {
    async fn start_function_instance(&mut self, spawn_request: SpawnFunctionRequest) -> anyhow::Result<FunctionId>;
    async fn stop_function_instance(&mut self, id: FunctionId) -> anyhow::Result<()>;
    async fn update_function_instance_links(&mut self, update: UpdateFunctionLinksRequest) -> anyhow::Result<()>;
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
