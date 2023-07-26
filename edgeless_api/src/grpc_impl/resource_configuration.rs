pub struct ResourceConfigurationConverters {}

impl ResourceConfigurationConverters {
    pub fn parse_api_instance_specification(
        api_spec: &crate::grpc_impl::api::ResourceInstanceSpecification,
    ) -> anyhow::Result<crate::resource_configuration::ResourceInstanceSpecification> {
        Ok(crate::resource_configuration::ResourceInstanceSpecification {
            provider_id: api_spec.provider_id.clone(),
            configuration: api_spec.configuration.clone(),
            output_callback_definitions: api_spec
                .output_callback_definitions
                .iter()
                .flat_map(|(alias, id)| {
                    let id = crate::grpc_impl::function_instance::FunctonInstanceConverters::parse_function_id(id);
                    match id {
                        Ok(val) => Some((alias.to_string(), val)),
                        Err(_) => None,
                    }
                })
                .collect(),
        })
    }

    pub fn serialize_crate_instance_specification(
        crate_spec: &crate::resource_configuration::ResourceInstanceSpecification,
    ) -> crate::grpc_impl::api::ResourceInstanceSpecification {
        crate::grpc_impl::api::ResourceInstanceSpecification {
            provider_id: crate_spec.provider_id.clone(),
            configuration: crate_spec.configuration.clone(),
            output_callback_definitions: crate_spec
                .output_callback_definitions
                .iter()
                .map(|(alias, id)| {
                    (
                        alias.to_string(),
                        crate::grpc_impl::function_instance::FunctonInstanceConverters::serialize_function_id(id),
                    )
                })
                .collect(),
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
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::resource_configuration::ResourceConfigurationAPI for ResourceConfigurationClient {
    async fn start_resource_instance(
        &mut self,
        instance_specification: crate::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<crate::function_instance::FunctionId> {
        let encoded = ResourceConfigurationConverters::serialize_crate_instance_specification(&instance_specification);
        match self.client.start_resource_instance(encoded).await {
            Ok(res) => {
                let decoded_id = crate::grpc_impl::function_instance::FunctonInstanceConverters::parse_function_id(&res.into_inner())?;
                Ok(decoded_id)
            }
            Err(err) => Err(anyhow::anyhow!("Resource Configuration Request Failed: {}", err)),
        }
    }

    async fn stop_resource_instance(&mut self, resource_id: crate::function_instance::FunctionId) -> anyhow::Result<()> {
        let encoded_id = crate::grpc_impl::function_instance::FunctonInstanceConverters::serialize_function_id(&resource_id);
        match self.client.stop_resource_instance(encoded_id).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Resource Configuration Request Failed: {}", err)),
        }
    }
}

pub struct ResourceConfigurationServerHandler {
    pub root_api: tokio::sync::Mutex<Box<dyn crate::resource_configuration::ResourceConfigurationAPI>>,
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::resource_configuration_server::ResourceConfiguration for ResourceConfigurationServerHandler {
    async fn start_resource_instance(
        &self,
        request: tonic::Request<crate::grpc_impl::api::ResourceInstanceSpecification>,
    ) -> tonic::Result<tonic::Response<crate::grpc_impl::api::FunctionId>> {
        let inner = request.into_inner();
        let parsed_spec = match crate::grpc_impl::resource_configuration::ResourceConfigurationConverters::parse_api_instance_specification(&inner) {
            Ok(val) => val,
            Err(_) => {
                return Err(tonic::Status::invalid_argument("Invalid ResourceInstance Specification"));
            }
        };
        let res = match self.root_api.lock().await.start_resource_instance(parsed_spec).await {
            Ok(val) => val,
            Err(_) => {
                return Err(tonic::Status::internal("Start ResourceInstance Failed"));
            }
        };
        Ok(tonic::Response::new(
            crate::grpc_impl::function_instance::FunctonInstanceConverters::serialize_function_id(&res),
        ))
    }

    async fn stop_resource_instance(&self, request: tonic::Request<crate::grpc_impl::api::FunctionId>) -> tonic::Result<tonic::Response<()>> {
        let inner = request.into_inner();
        let parsed_id = match crate::grpc_impl::function_instance::FunctonInstanceConverters::parse_function_id(&inner) {
            Ok(val) => val,
            Err(_) => {
                return Err(tonic::Status::invalid_argument("Invalid ResourceId"));
            }
        };
        let _res = match self.root_api.lock().await.stop_resource_instance(parsed_id).await {
            Ok(val) => val,
            Err(_) => {
                return Err(tonic::Status::internal("Stop ResourceInstance Failed"));
            }
        };
        Ok(tonic::Response::new(()))
    }
}

pub struct ResourceConfigurationServer {}

impl ResourceConfigurationServer {
    pub fn run(
        root_api: Box<dyn crate::resource_configuration::ResourceConfigurationAPI>,
        listen_addr: String,
    ) -> futures::future::BoxFuture<'static, ()> {
        let function_api = crate::grpc_impl::resource_configuration::ResourceConfigurationServerHandler {
            root_api: tokio::sync::Mutex::new(root_api),
        };
        Box::pin(async move {
            let function_api = function_api;
            if let Ok((_proto, host, port)) = crate::util::parse_http_host(&listen_addr) {
                if let Ok(host) = format!("{}:{}", host, port).parse() {
                    log::info!("Start ResourceConfiguration GRPC Server");
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
