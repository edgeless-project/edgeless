use edgeless_api_core::instance_id::ComponentId;

use crate::common::PatchRequest;

#[derive(Debug)]
pub struct ResourceInstanceSpecification {
    pub provider_id: String,
    pub output_mapping: std::collections::HashMap<String, crate::function_instance::InstanceId>,
    pub configuration: std::collections::HashMap<String, String>,
}

#[async_trait::async_trait]
pub trait ResourceConfigurationAPI: ResourceConfigurationAPIClone + Sync + Send {
    async fn start(&mut self, instance_specification: ResourceInstanceSpecification) -> anyhow::Result<crate::common::StartComponentResponse>;
    async fn stop(&mut self, resource_id: crate::function_instance::InstanceId) -> anyhow::Result<()>;
    async fn patch(&mut self, update: PatchRequest) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct MultiResouceConfigurationAPI {
    // key: provider_id
    // value: resource configuration API
    pub resource_providers: std::collections::HashMap<String, Box<dyn ResourceConfigurationAPI>>,
    // key: fid
    // value: provider_id
    pub resource_instances: std::collections::HashMap<ComponentId, String>,
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
                self.resource_instances.insert(id.function_id.clone(), provider_id.clone());
                Ok(crate::common::StartComponentResponse::InstanceId(id))
            } else {
                Ok(res)
            }
        } else {
            Ok(crate::common::StartComponentResponse::ResponseError(crate::common::ResponseError {
                summary: "Error when creating a resource".to_string(),
                detail: Some(format!("Provider does not exist: {}", instance_specification.provider_id)),
            }))
        }
    }

    async fn stop(&mut self, resource_id: crate::function_instance::InstanceId) -> anyhow::Result<()> {
        if let Some(provider_id) = self.resource_instances.get(&resource_id.function_id) {
            if let Some(provider) = self.resource_providers.get_mut(provider_id) {
                log::info!(
                    "Stopped resource provider_id {} node_id {}, fid {}",
                    provider_id,
                    resource_id.node_id,
                    resource_id.function_id
                );
                return provider.stop(resource_id).await;
            } else {
                return Err(anyhow::anyhow!(
                    "Cannot stop a resource, provider not found with provider_id: {}",
                    provider_id
                ));
            }
        }
        Err(anyhow::anyhow!("Cannot stop a resource, not found with fid: {}", resource_id.function_id))
    }

    async fn patch(&mut self, update: PatchRequest) -> anyhow::Result<()> {
        if let Some(provider_id) = self.resource_instances.get(&update.function_id) {
            if let Some(provider) = self.resource_providers.get_mut(provider_id) {
                log::info!("Patch resource provider_id {} fid {}", provider_id, update.function_id);
                return provider.patch(update).await;
            } else {
                return Err(anyhow::anyhow!(
                    "Cannot patch a resource, provider not found with provider_id: {}",
                    provider_id
                ));
            }
        }
        Err(anyhow::anyhow!("Cannot patch a resource, not found with fid: {}", update.function_id))
    }
}

// https://stackoverflow.com/a/30353928
pub trait ResourceConfigurationAPIClone {
    fn clone_box(&self) -> Box<dyn ResourceConfigurationAPI>;
}
impl<T> ResourceConfigurationAPIClone for T
where
    T: 'static + ResourceConfigurationAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn ResourceConfigurationAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn ResourceConfigurationAPI> {
    fn clone(&self) -> Box<dyn ResourceConfigurationAPI> {
        self.clone_box()
    }
}
