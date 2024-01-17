// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use super::common::{CommonConverters, ParseableId, SerializeableId};

pub struct ResourceConfigurationConverters {}

impl ResourceConfigurationConverters {
    pub fn parse_resource_instance_specification(
        api_spec: &crate::grpc_impl::api::ResourceInstanceSpecification,
    ) -> anyhow::Result<crate::resource_configuration::ResourceInstanceSpecification> {
        Ok(crate::resource_configuration::ResourceInstanceSpecification {
            class_type: api_spec.resource_class_type.clone(),
            configuration: api_spec.configuration.clone(),
            output_mapping: api_spec
                .output_mapping
                .iter()
                .flat_map(|(name, id)| {
                    let id = CommonConverters::parse_instance_id(id);
                    match id {
                        Ok(val) => Some((name.to_string(), val)),
                        Err(_) => None,
                    }
                })
                .collect(),
        })
    }

    pub fn serialize_resource_instance_specification(
        crate_spec: &crate::resource_configuration::ResourceInstanceSpecification,
    ) -> crate::grpc_impl::api::ResourceInstanceSpecification {
        crate::grpc_impl::api::ResourceInstanceSpecification {
            resource_class_type: crate_spec.class_type.clone(),
            configuration: crate_spec.configuration.clone(),
            output_mapping: crate_spec
                .output_mapping
                .iter()
                .map(|(name, id)| (name.to_string(), CommonConverters::serialize_instance_id(id)))
                .collect(),
        }
    }
}

#[derive(Clone)]
pub struct ResourceConfigurationClient<ResourceIdType> {
    client: Option<crate::grpc_impl::api::resource_configuration_client::ResourceConfigurationClient<tonic::transport::Channel>>,
    _phantom: std::marker::PhantomData<ResourceIdType>,
}

impl<ResourceIdType> ResourceConfigurationClient<ResourceIdType> {
    pub async fn new(server_addr: &str, retry_interval: Option<u64>) -> Self {
        loop {
            match crate::grpc_impl::api::resource_configuration_client::ResourceConfigurationClient::connect(server_addr.to_string()).await {
                Ok(client) => {
                    let client = client.max_decoding_message_size(usize::MAX);
                    return Self {
                        client: Some(client),
                        _phantom: std::marker::PhantomData {},
                    };
                }
                Err(err) => match retry_interval {
                    Some(val) => tokio::time::sleep(tokio::time::Duration::from_secs(val)).await,
                    None => {
                        log::warn!("Error when connecting to {}: {}", server_addr, err);
                        return Self {
                            client: None,
                            _phantom: std::marker::PhantomData {},
                        };
                    }
                },
            }
        }
    }
}

#[async_trait::async_trait]
impl<ResourceIdType: SerializeableId + Clone + Send + Sync + 'static> crate::resource_configuration::ResourceConfigurationAPI<ResourceIdType>
    for ResourceConfigurationClient<ResourceIdType>
where
    super::api::InstanceIdVariant: ParseableId<ResourceIdType>,
{
    async fn start(
        &mut self,
        instance_specification: crate::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<crate::common::StartComponentResponse<ResourceIdType>> {
        match &mut self.client {
            Some(client) => {
                let serialized_request = ResourceConfigurationConverters::serialize_resource_instance_specification(&instance_specification);
                match client.start(tonic::Request::new(serialized_request)).await {
                    Ok(ret) => CommonConverters::parse_start_component_response(&ret.into_inner()),
                    Err(err) => Err(anyhow::anyhow!("Resource configuration request failed: {}", err)),
                }
            }
            None => {
                return Err(anyhow::anyhow!("Resource configuration not connected"));
            }
        }
    }

    async fn stop(&mut self, resource_id: ResourceIdType) -> anyhow::Result<()> {
        match &mut self.client {
            Some(client) => {
                let encoded_id = SerializeableId::serialize(&resource_id);
                match client.stop(encoded_id).await {
                    Ok(_) => Ok(()),
                    Err(err) => Err(anyhow::anyhow!("Resource stop request failed: {}", err)),
                }
            }
            None => {
                return Err(anyhow::anyhow!("Resource configuration not connected"));
            }
        }
    }

    async fn patch(&mut self, update: crate::common::PatchRequest) -> anyhow::Result<()> {
        match &mut self.client {
            Some(client) => {
                let encoded_request = CommonConverters::serialize_patch_request(&update);
                match client.patch(encoded_request).await {
                    Ok(_) => Ok(()),
                    Err(err) => Err(anyhow::anyhow!("Resource patch request failed: {}", err)),
                }
            }
            None => {
                return Err(anyhow::anyhow!("Resource configuration not connected"));
            }
        }
    }
}

pub struct ResourceConfigurationServerHandler<ResourceIdType> {
    pub root_api: tokio::sync::Mutex<Box<dyn crate::resource_configuration::ResourceConfigurationAPI<ResourceIdType>>>,
}

#[async_trait::async_trait]
impl<ResourceIdType: Clone + Send + 'static> crate::grpc_impl::api::resource_configuration_server::ResourceConfiguration
    for ResourceConfigurationServerHandler<ResourceIdType>
where
    crate::grpc_impl::api::InstanceIdVariant: crate::grpc_impl::common::ParseableId<ResourceIdType>,
    ResourceIdType: crate::grpc_impl::common::SerializeableId,
{
    async fn start(
        &self,
        request: tonic::Request<crate::grpc_impl::api::ResourceInstanceSpecification>,
    ) -> tonic::Result<tonic::Response<crate::grpc_impl::api::StartComponentResponse>> {
        let inner = request.into_inner();
        let parsed_spec =
            match crate::grpc_impl::resource_configuration::ResourceConfigurationConverters::parse_resource_instance_specification(&inner) {
                Ok(val) => val,
                Err(err) => {
                    return Ok(tonic::Response::new(crate::grpc_impl::api::StartComponentResponse {
                        response_error: Some(crate::grpc_impl::api::ResponseError {
                            summary: "Invalid resource specification".to_string(),
                            detail: Some(err.to_string()),
                        }),
                        instance_id: None,
                    }))
                }
            };
        match self.root_api.lock().await.start(parsed_spec).await {
            Ok(response) => Ok(tonic::Response::new(CommonConverters::serialize_start_component_response(&response))),
            Err(err) => {
                return Ok(tonic::Response::new(crate::grpc_impl::api::StartComponentResponse {
                    response_error: Some(crate::grpc_impl::api::ResponseError {
                        summary: "Resource creation rejected".to_string(),
                        detail: Some(err.to_string()),
                    }),
                    instance_id: None,
                }))
            }
        }
    }

    async fn stop(&self, request: tonic::Request<crate::grpc_impl::api::InstanceIdVariant>) -> tonic::Result<tonic::Response<()>> {
        let inner: super::api::InstanceIdVariant = request.into_inner();
        let parsed_id = match crate::grpc_impl::common::ParseableId::<ResourceIdType>::parse(&inner) {
            Ok(val) => val,
            Err(err) => {
                return Err(tonic::Status::invalid_argument(format!("Error when deleting a resource: {}", err)));
            }
        };
        match self.root_api.lock().await.stop(parsed_id).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Error when deleting a resource: {}", err))),
        }
    }

    async fn patch(&self, update: tonic::Request<crate::grpc_impl::api::PatchRequest>) -> tonic::Result<tonic::Response<()>> {
        let inner = update.into_inner();
        let parsed_request = match CommonConverters::parse_patch_request(&inner) {
            Ok(val) => val,
            Err(err) => {
                return Err(tonic::Status::invalid_argument(format!("Error when patching a resource: {}", err)));
            }
        };
        match self.root_api.lock().await.patch(parsed_request).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Error when patching a resource: {}", err))),
        }
    }
}
