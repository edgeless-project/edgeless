#[async_trait::async_trait]
pub trait RunnerAPI {
    async fn start(&mut self, function_id: edgeless_api::function_instance::SpawnFunctionRequest) -> anyhow::Result<()>;
    async fn stop(&mut self, function_id: edgeless_api::function_instance::FunctionId) -> anyhow::Result<()>;
    async fn update(&mut self, update: edgeless_api::function_instance::UpdateFunctionLinksRequest) -> anyhow::Result<()>;
}
