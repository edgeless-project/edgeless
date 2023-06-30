pub trait OrchestratorAPI {
    fn function_instance_api(&mut self) -> Box<dyn crate::function_instance::FunctionInstanceAPI + Send>;
}
