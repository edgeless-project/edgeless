// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use crate::grpc_impl::api as grpc_stubs;
use crate::grpc_impl::common::{CommonConverters, ParseableId, SerializeableId};

#[derive(Clone)]
pub struct ResourceConfigurationClient<ResourceIdType> {
    client: Option<crate::grpc_impl::api::resource_configuration_client::ResourceConfigurationClient<tonic::transport::Channel>>,
    server_addr: String,
    _phantom: std::marker::PhantomData<ResourceIdType>,
    tls_config: Option<crate::grpc_impl::tls_config::TlsConfig>,
}

impl<ResourceIdType> ResourceConfigurationClient<ResourceIdType> {
    pub fn new(server_addr: String, tls_config: Option<crate::grpc_impl::tls_config::TlsConfig>) -> Self {
        Self {
            client: None,
            server_addr,
            _phantom: std::marker::PhantomData {},
            tls_config,
        }
    }

    /// Try connecting, if not already connected.
    ///
    /// If an error is returned, then the client is set to None (disconnected).
    /// Otherwise, the client is set to some value (connected).
    async fn try_connect(&mut self) -> anyhow::Result<()> {
        if self.client.is_none() {
            let tls_config = if let Some(config) = &self.tls_config {
                config.clone()
            } else {
                crate::grpc_impl::tls_config::TlsConfig::global_client().clone()
            };

            let channel = tls_config.create_client_channel(&self.server_addr).await?;

            self.client = Some(
                crate::grpc_impl::api::resource_configuration_client::ResourceConfigurationClient::new(channel).max_decoding_message_size(usize::MAX),
            );
        }
        Ok(())
    }

    /// Disconnect the client.
    fn disconnect(&mut self) {
        self.client = None;
    }
}

#[async_trait::async_trait]
impl<ResourceIdType: SerializeableId + Clone + Send + Sync + 'static> crate::resource_configuration::ResourceConfigurationAPI<ResourceIdType>
    for ResourceConfigurationClient<ResourceIdType>
where
    grpc_stubs::InstanceIdVariant: ParseableId<ResourceIdType>,
{
    async fn start(
        &mut self,
        instance_specification: crate::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<crate::common::StartComponentResponse<ResourceIdType>> {
        match self.try_connect().await {
            Ok(_) => {
                if let Some(client) = &mut self.client {
                    match client
                        .start(tonic::Request::new(serialize_resource_instance_specification(&instance_specification)))
                        .await
                    {
                        Ok(res) => CommonConverters::parse_start_component_response(&res.into_inner()),
                        Err(err) => {
                            self.disconnect();
                            Err(anyhow::anyhow!(
                                "Error when starting a resource at {}: {}",
                                self.server_addr,
                                err.to_string()
                            ))
                        }
                    }
                } else {
                    panic!("The impossible happened");
                }
            }
            Err(err) => {
                anyhow::bail!("Error when connecting to {}: {}", self.server_addr, err);
            }
        }
    }

    async fn stop(&mut self, resource_id: ResourceIdType) -> anyhow::Result<()> {
        match self.try_connect().await {
            Ok(_) => {
                if let Some(client) = &mut self.client {
                    match client.stop(SerializeableId::serialize(&resource_id)).await {
                        Ok(_) => Ok(()),
                        Err(err) => {
                            self.disconnect();
                            Err(anyhow::anyhow!(
                                "Error when stopping a resource at {}: {}",
                                self.server_addr,
                                err.to_string()
                            ))
                        }
                    }
                } else {
                    panic!("The impossible happened");
                }
            }
            Err(err) => {
                anyhow::bail!("Error when connecting to {}: {}", self.server_addr, err);
            }
        }
    }

    async fn patch(&mut self, update: crate::common::PatchRequest) -> anyhow::Result<()> {
        match self.try_connect().await {
            Ok(_) => {
                if let Some(client) = &mut self.client {
                    match client.patch(CommonConverters::serialize_patch_request(&update)).await {
                        Ok(_) => Ok(()),
                        Err(err) => {
                            self.disconnect();
                            Err(anyhow::anyhow!(
                                "Error when patching a resource at {}: {}",
                                self.server_addr,
                                err.to_string()
                            ))
                        }
                    }
                } else {
                    panic!("The impossible happened");
                }
            }
            Err(err) => {
                anyhow::bail!("Error when connecting to {}: {}", self.server_addr, err);
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
        let parsed_spec = match super::resource_configuration::parse_resource_instance_specification(&inner) {
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
        let inner: grpc_stubs::InstanceIdVariant = request.into_inner();
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

fn parse_resource_instance_specification(
    api_spec: &crate::grpc_impl::api::ResourceInstanceSpecification,
) -> anyhow::Result<crate::resource_configuration::ResourceInstanceSpecification> {
    Ok(crate::resource_configuration::ResourceInstanceSpecification {
        class_type: api_spec.resource_class_type.clone(),
        configuration: api_spec.configuration.clone(),
        workflow_id: api_spec.workflow_id.clone(),
    })
}

fn serialize_resource_instance_specification(
    crate_spec: &crate::resource_configuration::ResourceInstanceSpecification,
) -> crate::grpc_impl::api::ResourceInstanceSpecification {
    crate::grpc_impl::api::ResourceInstanceSpecification {
        resource_class_type: crate_spec.class_type.clone(),
        configuration: crate_spec.configuration.clone(),
        workflow_id: crate_spec.workflow_id.clone(),
    }
}
