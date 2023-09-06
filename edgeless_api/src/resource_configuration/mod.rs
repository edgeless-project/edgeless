// struct ResourceProviderSpecification {
//     resource_provider_id: String,
//     resource_class_type: String,
//     output_callback_declarations: Vec<String>,
// }

pub struct ResourceInstanceSpecification {
    pub provider_id: String,
    pub output_callback_definitions: std::collections::HashMap<String, crate::function_instance::InstanceId>,
    pub configuration: std::collections::HashMap<String, String>,
}

#[async_trait::async_trait]
pub trait ResourceConfigurationAPI: Sync + Send {
    async fn start(&mut self, instance_specification: ResourceInstanceSpecification) -> anyhow::Result<crate::function_instance::InstanceId>;
    async fn stop(&mut self, resource_id: crate::function_instance::InstanceId) -> anyhow::Result<()>;
}

pub struct MultiResouceConfigurationAPI {
    pub resource_providers: std::collections::HashMap<String, Box<dyn ResourceConfigurationAPI>>,
    pub resource_instances: std::collections::HashMap<crate::function_instance::InstanceId, String>,
}

impl MultiResouceConfigurationAPI {
    pub fn new(resource_providers: std::collections::HashMap<String, Box<dyn ResourceConfigurationAPI>>) -> Self {
        Self {
            resource_providers,
            resource_instances: std::collections::HashMap::new(),
        }
    }
}

#[async_trait::async_trait]
impl ResourceConfigurationAPI for MultiResouceConfigurationAPI {
    async fn start(&mut self, instance_specification: ResourceInstanceSpecification) -> anyhow::Result<crate::function_instance::InstanceId> {
        if let Some(resource) = self.resource_providers.get_mut(&instance_specification.provider_id) {
            let provider = instance_specification.provider_id.clone();
            let id = resource.start(instance_specification).await?;
            self.resource_instances.insert(id.clone(), provider.clone());
            Ok(id)
        } else {
            return Err(anyhow::anyhow!("Resource provider does not exist"));
        }
    }

    async fn stop(&mut self, resource_id: crate::function_instance::InstanceId) -> anyhow::Result<()> {
        if let Some(instance_id) = self.resource_instances.get(&resource_id) {
            if let Some(provider) = self.resource_providers.get_mut(instance_id) {
                return provider.stop(resource_id).await;
            }
        }
        Err(anyhow::anyhow!("Could not delete. (Missing?)"))
    }
}
