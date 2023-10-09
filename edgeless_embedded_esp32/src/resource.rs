pub trait Resource<'a, ResourceInstanceSpecification>:
    edgeless_api_core::invocation::InvocationAPI + edgeless_api_core::resource_configuration::ResourceConfigurationAPI<'a, ResourceInstanceSpecification>
{
    fn provider_id(&self) -> &'static str;
    async fn has_instance(&self, id: &edgeless_api_core::instance_id::InstanceId) -> bool;
}
