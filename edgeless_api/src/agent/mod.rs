pub trait AgentAPI {
    fn function_instance_api(&mut self) -> Box<dyn crate::function_instance::FunctionInstanceNodeAPI>;
    fn node_management_api(&mut self) -> Box<dyn crate::node_managment::NodeManagementAPI>;
    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn crate::resource_configuration::ResourceConfigurationAPI<edgeless_api_core::instance_id::InstanceId>>;
}
