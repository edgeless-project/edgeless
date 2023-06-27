#[cfg(feature = "grpc_impl")]
pub mod grpc_impl;

#[derive(Debug, Clone)]
pub struct FunctionId {
    node_id: uuid::Uuid,
    function_id: uuid::Uuid,
}

impl FunctionId {
    pub fn new(node_id: uuid::Uuid) -> Self {
        Self {
            node_id: node_id,
            function_id: uuid::Uuid::new_v4(),
        }
    }
}

#[derive(Debug)]
pub struct FunctionClassSpecification {
    pub function_class_id: String,
    pub function_class_type: String,
    pub function_class_version: String,
    pub function_class_inlude_code: Vec<u8>,
    pub output_callback_declarations: Vec<String>,
}

#[derive(Debug)]
pub struct SpawnFunctionRequest {
    pub function_id: Option<FunctionId>,
    pub code: FunctionClassSpecification,
    pub output_callback_definitions: std::collections::HashMap<String, FunctionId>,
    pub return_continuation: FunctionId,
    pub annotations: std::collections::HashMap<String, String>,
}

#[async_trait::async_trait]
pub trait AgentAPI: Sync {
    async fn start_function_instance(&mut self, request: SpawnFunctionRequest) -> anyhow::Result<FunctionId>;
    async fn stop_function_instance(&mut self, id: FunctionId) -> anyhow::Result<()>;
}
