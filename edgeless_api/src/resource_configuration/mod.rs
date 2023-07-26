// struct ResourceProviderSpecification {
//     resource_provider_id: String,
//     resource_class_type: String,
//     output_callback_declarations: Vec<String>,
// }

pub struct ResourceInstanceSpecification {
    pub provider_id: String,
    pub output_callback_definitions: std::collections::HashMap<String, crate::function_instance::FunctionId>,
    pub configuration: std::collections::HashMap<String, String>,
}

#[async_trait::async_trait]
pub trait ResourceConfigurationAPI: Sync + Send {
    async fn start_resource_instance(
        &mut self,
        instance_specification: ResourceInstanceSpecification,
    ) -> anyhow::Result<crate::function_instance::FunctionId>;
    async fn stop_resource_instance(&mut self, resource_id: crate::function_instance::FunctionId) -> anyhow::Result<()>;
}
