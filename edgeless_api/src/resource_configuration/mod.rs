#[derive(Debug)]
pub struct ResourceInstanceSpecification {
    pub provider_id: String,
    pub output_mapping: std::collections::HashMap<String, crate::function_instance::InstanceId>,
    pub configuration: std::collections::HashMap<String, String>,
}

#[async_trait::async_trait]
pub trait ResourceConfigurationAPI: Sync + Send {
    async fn start(&mut self, instance_specification: ResourceInstanceSpecification) -> anyhow::Result<crate::common::StartComponentResponse>;
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
    async fn start(&mut self, instance_specification: ResourceInstanceSpecification) -> anyhow::Result<crate::common::StartComponentResponse> {
        if let Some(resource) = self.resource_providers.get_mut(&instance_specification.provider_id) {
            let provider_id = instance_specification.provider_id.clone();
            let res = resource.start(instance_specification).await?;
            if let crate::common::StartComponentResponse::InstanceId(id) = res {
                log::info!(
                    "Started resource provider_id {}, node_id {}, fid {}",
                    provider_id,
                    id.node_id,
                    id.function_id
                );
                self.resource_instances.insert(id.clone(), provider_id.clone());
                Ok(crate::common::StartComponentResponse::InstanceId(id))
            } else {
                Ok(res)
            }
        } else {
            Ok(crate::common::StartComponentResponse::ResponseError(crate::common::ResponseError {
                summary: "Error when creating a resource".to_string(),
                detail: Some("Provider does not exist".to_string()),
            }))
        }
    }

    async fn stop(&mut self, resource_id: crate::function_instance::InstanceId) -> anyhow::Result<()> {
        if let Some(instance_id) = self.resource_instances.get(&resource_id) {
            if let Some(provider) = self.resource_providers.get_mut(instance_id) {
                log::info!("Stopped resource node_id {}, fid {}", resource_id.node_id, resource_id.function_id);
                return provider.stop(resource_id).await;
            }
        }
        Err(anyhow::anyhow!("Error when deleting a resource: the resource may not exist"))
    }
}
