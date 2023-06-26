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
pub struct SpawnFunctionRequest {
    pub function_id: Option<FunctionId>,
    pub code: String,
}

#[async_trait::async_trait]
pub trait AgentAPI: Sync {
    async fn spawn(&mut self, request: SpawnFunctionRequest) -> anyhow::Result<FunctionId>;
    async fn stop(&mut self, id: FunctionId) -> anyhow::Result<()>;
}
