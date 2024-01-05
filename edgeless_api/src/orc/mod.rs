pub trait OrchestratorAPI: Send {
    fn function_instance_api(&mut self) -> Box<dyn crate::function_instance::FunctionInstanceOrcAPI>;
    fn node_registration_api(&mut self) -> Box<dyn crate::node_registration::NodeRegistrationAPI>;
}
