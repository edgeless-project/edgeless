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

    pub fn parse_spawn_resource_response(
        api_instance: &crate::grpc_impl::api::SpawnResourceResponse,
    ) -> anyhow::Result<crate::resource_configuration::SpawnResourceResponse> {
        match api_instance.instance_id.as_ref() {
            Some(val) => match CommonConverters::parse_instance_id(val) {
                Ok(val) => Ok(crate::resource_configuration::SpawnResourceResponse::InstanceId(val)),
                Err(err) => Err(anyhow::anyhow!(err.to_string())),
            },
            None => match api_instance.response_error.as_ref() {
                Some(val) => match CommonConverters::parse_response_error(val) {
                    Ok(val) => Ok(crate::resource_configuration::SpawnResourceResponse::ResponseError(val)),
                    Err(err) => Err(anyhow::anyhow!(err.to_string())),
                },
                None => Err(anyhow::anyhow!(
                    "Ill-formed SpawnResourceResponse message: both ResponseError and InstanceId are empty"
                )),
            },
        }
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

    pub fn serialize_spawn_resource_response(
        req: &crate::resource_configuration::SpawnResourceResponse,
    ) -> crate::grpc_impl::api::SpawnResourceResponse {
        match req {
            crate::resource_configuration::SpawnResourceResponse::ResponseError(err) => crate::grpc_impl::api::SpawnResourceResponse {
                response_error: Some(CommonConverters::serialize_response_error(&err)),
                instance_id: None,
            },
            crate::resource_configuration::SpawnResourceResponse::InstanceId(id) => crate::grpc_impl::api::SpawnResourceResponse {
                response_error: None,
                instance_id: Some(CommonConverters::serialize_instance_id(&id)),
            },
        }
    }
}

pub struct ResourceConfigurationClient {
    client: crate::grpc_impl::api::resource_configuration_client::ResourceConfigurationClient<tonic::transport::Channel>,
}

impl ResourceConfigurationClient {
    pub async fn new(server_addr: &str) -> Self {
        loop {
            match crate::grpc_impl::api::resource_configuration_client::ResourceConfigurationClient::connect(server_addr.to_string()).await {
                Ok(client) => {
                    let client = client.max_decoding_message_size(usize::MAX);
                    return Self { client };
                }
                Err(_) => {
                    log::warn!("could not connect to {:?}, retrying", server_addr);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::resource_configuration::ResourceConfigurationAPI for ResourceConfigurationClient {
    async fn start(
        &mut self,
        instance_specification: crate::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<crate::resource_configuration::SpawnResourceResponse> {
        let serialized_request = ResourceConfigurationConverters::serialize_resource_instance_specification(&instance_specification);
        match self.client.start(tonic::Request::new(serialized_request)).await {
            Ok(ret) => ResourceConfigurationConverters::parse_spawn_resource_response(&ret.into_inner()),
            Err(err) => Err(anyhow::anyhow!("Resource configuration request failed: {}", err)),
        }
    }

    async fn stop(&mut self, resource_id: crate::function_instance::InstanceId) -> anyhow::Result<()> {
        let encoded_id = CommonConverters::serialize_instance_id(&resource_id);
        match self.client.stop(encoded_id).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Resource configuration request failed: {}", err)),
        }
    }
}

pub struct ResourceConfigurationServerHandler {
    pub root_api: tokio::sync::Mutex<Box<dyn crate::resource_configuration::ResourceConfigurationAPI>>,
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::resource_configuration_server::ResourceConfiguration for ResourceConfigurationServerHandler {
    async fn start(
        &self,
        request: tonic::Request<crate::grpc_impl::api::ResourceInstanceSpecification>,
    ) -> tonic::Result<tonic::Response<crate::grpc_impl::api::SpawnResourceResponse>> {
        let inner = request.into_inner();
        let parsed_spec =
            match crate::grpc_impl::resource_configuration::ResourceConfigurationConverters::parse_resource_instance_specification(&inner) {
                Ok(val) => val,
                Err(err) => {
                    return Ok(tonic::Response::new(crate::grpc_impl::api::SpawnResourceResponse {
                        response_error: Some(crate::grpc_impl::api::ResponseError {
                            summary: "Invalid resource specification".to_string(),
                            detail: Some(err.to_string()),
                        }),
                        instance_id: None,
                    }))
                }
            };
        match self.root_api.lock().await.start(parsed_spec).await {
            Ok(response) => Ok(tonic::Response::new(ResourceConfigurationConverters::serialize_spawn_resource_response(
                &response,
            ))),
            Err(err) => {
                return Ok(tonic::Response::new(crate::grpc_impl::api::SpawnResourceResponse {
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
}

pub struct ResourceConfigurationServer {}

impl ResourceConfigurationServer {
    pub fn run(
        root_api: Box<dyn crate::resource_configuration::ResourceConfigurationAPI>,
        resource_configuration_url: String,
    ) -> futures::future::BoxFuture<'static, ()> {
        let function_api = crate::grpc_impl::resource_configuration::ResourceConfigurationServerHandler {
            root_api: tokio::sync::Mutex::new(root_api),
        };
        Box::pin(async move {
            let function_api = function_api;
            if let Ok((_proto, host, port)) = crate::util::parse_http_host(&resource_configuration_url) {
                if let Ok(host) = format!("{}:{}", host, port).parse() {
                    log::info!("Start ResourceConfiguration GRPC Server at {}", resource_configuration_url);
                    match tonic::transport::Server::builder()
                        .add_service(
                            crate::grpc_impl::api::resource_configuration_server::ResourceConfigurationServer::new(function_api)
                                .max_decoding_message_size(usize::MAX),
                        )
                        .serve(host)
                        .await
                    {
                        Ok(_) => {
                            log::debug!("Clean Exit");
                        }
                        Err(_) => {
                            log::error!("GRPC ResourceConfiguration Failure");
                        }
                    }
                }
            }

            log::info!("Stop ResourceConfiguration GRPC Server");
        })
    }
}
