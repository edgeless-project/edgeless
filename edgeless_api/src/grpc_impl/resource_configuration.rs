use super::common::CommonConverters;

pub struct ResourceConfigurationConverters {}

impl ResourceConfigurationConverters {
    pub fn parse_resource_instance_specification(
        api_spec: &crate::grpc_impl::api::ResourceInstanceSpecification,
    ) -> anyhow::Result<crate::resource_configuration::ResourceInstanceSpecification> {
        Ok(crate::resource_configuration::ResourceInstanceSpecification {
            provider_id: api_spec.provider_id.clone(),
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
            provider_id: crate_spec.provider_id.clone(),
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
pub struct ResourceConfigurationAPIClient {
    client: Option<crate::grpc_impl::api::resource_configuration_client::ResourceConfigurationClient<tonic::transport::Channel>>,
}

impl ResourceConfigurationAPIClient {
    pub async fn new(server_addr: &str, no_retry: bool) -> Self {
        loop {
            match crate::grpc_impl::api::resource_configuration_client::ResourceConfigurationClient::connect(server_addr.to_string()).await {
                Ok(client) => {
                    let client = client.max_decoding_message_size(usize::MAX);
                    return Self { client: Some(client) };
                }
                Err(_) => {
                    if no_retry {
                        return Self { client: None };
                    }
                    log::warn!("could not connect to {:?}, retrying", server_addr);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::resource_configuration::ResourceConfigurationAPI for ResourceConfigurationAPIClient {
    async fn start(
        &mut self,
        instance_specification: crate::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<crate::common::StartComponentResponse> {
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

    async fn stop(&mut self, resource_id: crate::function_instance::InstanceId) -> anyhow::Result<()> {
        match &mut self.client {
            Some(client) => {
                let encoded_id = CommonConverters::serialize_instance_id(&resource_id);
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

pub struct ResourceConfigurationAPIServer {
    pub root_api: tokio::sync::Mutex<Box<dyn crate::resource_configuration::ResourceConfigurationAPI>>,
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::resource_configuration_server::ResourceConfiguration for ResourceConfigurationAPIServer {
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

    async fn stop(&self, request: tonic::Request<crate::grpc_impl::api::InstanceId>) -> tonic::Result<tonic::Response<()>> {
        let inner = request.into_inner();
        let parsed_id = match CommonConverters::parse_instance_id(&inner) {
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
