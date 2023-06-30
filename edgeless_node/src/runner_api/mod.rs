#[async_trait::async_trait]
pub trait RunnerAPI {
    async fn start(&mut self, function_id: edgeless_api::function_instance::FunctionId);
    async fn stop(&mut self, function_id: edgeless_api::function_instance::FunctionId);
}
