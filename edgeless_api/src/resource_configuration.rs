use crate::common::PatchRequest;

#[derive(Debug, Clone)]
pub struct ResourceInstanceSpecification {
    pub class_type: String,
    pub output_mapping: std::collections::HashMap<String, crate::function_instance::InstanceId>,
    pub configuration: std::collections::HashMap<String, String>,
}

#[async_trait::async_trait]
pub trait ResourceConfigurationAPI<ResourceIdType: Clone>: ResourceConfigurationAPIClone<ResourceIdType> + Sync + Send {
    async fn start(
        &mut self,
        instance_specification: ResourceInstanceSpecification,
    ) -> anyhow::Result<crate::common::StartComponentResponse<ResourceIdType>>;
    async fn stop(&mut self, resource_id: ResourceIdType) -> anyhow::Result<()>;
    async fn patch(&mut self, update: PatchRequest) -> anyhow::Result<()>;
}

// https://stackoverflow.com/a/30353928
pub trait ResourceConfigurationAPIClone<ResourceIdType: Clone> {
    fn clone_box(&self) -> Box<dyn ResourceConfigurationAPI<ResourceIdType>>;
}

impl<T, ResourceIdType> ResourceConfigurationAPIClone<ResourceIdType> for T
where
    T: 'static + ResourceConfigurationAPI<ResourceIdType> + Clone,
    ResourceIdType: Clone,
{
    fn clone_box(&self) -> Box<dyn ResourceConfigurationAPI<ResourceIdType>> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn ResourceConfigurationAPI<crate::function_instance::InstanceId>> {
    fn clone(&self) -> Box<dyn ResourceConfigurationAPI<crate::function_instance::InstanceId>> {
        self.clone_box()
    }
}

impl Clone for Box<dyn ResourceConfigurationAPI<crate::orc::DomainManagedInstanceId>> {
    fn clone(&self) -> Box<dyn ResourceConfigurationAPI<crate::orc::DomainManagedInstanceId>> {
        self.clone_box()
    }
}
