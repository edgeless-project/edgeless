// struct ResourceProviderSpecification {
//     resource_provider_id: String,
//     class_type: String,
//     output_callback_declarations: Vec<String>,
// }

pub struct ResourceInstanceSpecification {
    pub provider_id: String,
    pub output_callback_definitions: std::collections::HashMap<String, crate::function_instance::InstanceId>,
    pub configuration: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum SpawnResourceResponse {
    ResponseError(crate::common::ResponseError),
    InstanceId(crate::function_instance::InstanceId),
}

#[async_trait::async_trait]
pub trait ResourceConfigurationAPI: Sync + Send {
    async fn start(&mut self, instance_specification: ResourceInstanceSpecification) -> anyhow::Result<SpawnResourceResponse>;
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
    async fn start(&mut self, instance_specification: ResourceInstanceSpecification) -> anyhow::Result<SpawnResourceResponse> {
        if let Some(resource) = self.resource_providers.get_mut(&instance_specification.provider_id) {
            let provider = instance_specification.provider_id.clone();
            let res = resource.start(instance_specification).await?;
            if let SpawnResourceResponse::InstanceId(id) = res {
                self.resource_instances.insert(id.clone(), provider.clone());
                Ok(SpawnResourceResponse::InstanceId(id))
            } else {
                Ok(res)
            }
        } else {
            Ok(SpawnResourceResponse::ResponseError(crate::common::ResponseError {
                summary: "Error when creating a resource".to_string(),
                detail: Some("Provider does not exist".to_string()),
            }))
        }
    }

    async fn stop(&mut self, resource_id: crate::function_instance::InstanceId) -> anyhow::Result<()> {
        if let Some(instance_id) = self.resource_instances.get(&resource_id) {
            if let Some(provider) = self.resource_providers.get_mut(instance_id) {
                return provider.stop(resource_id).await;
            }
        }
        Err(anyhow::anyhow!("Error when deleting a resource: the resource may not exist"))
    }
}
