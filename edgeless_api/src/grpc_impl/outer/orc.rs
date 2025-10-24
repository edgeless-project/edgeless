// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub struct OrchestratorAPIClient {
    function_instance_client: Box<dyn crate::function_instance::FunctionInstanceAPI<crate::function_instance::DomainManagedInstanceId>>,
    resource_configuration_client:
        Box<dyn crate::resource_configuration::ResourceConfigurationAPI<crate::function_instance::DomainManagedInstanceId>>,
}

impl OrchestratorAPIClient {
    pub async fn new(api_addr: &str, tls_config: Option<crate::grpc_impl::tls_config::TlsConfig>) -> anyhow::Result<Self> {
        Ok(Self {
            function_instance_client: Box::new(crate::grpc_impl::inner::function_instance::FunctionInstanceAPIClient::new(
                api_addr.to_string(),
                tls_config.clone(),
            )),
            resource_configuration_client: Box::new(crate::grpc_impl::inner::resource_configuration::ResourceConfigurationClient::new(
                api_addr.to_string(),
                tls_config.clone(),
            )),
        })
    }
}

impl crate::outer::orc::OrchestratorAPI for OrchestratorAPIClient {
    fn function_instance_api(&mut self) -> Box<dyn crate::function_instance::FunctionInstanceAPI<crate::function_instance::DomainManagedInstanceId>> {
        self.function_instance_client.clone()
    }

    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn crate::resource_configuration::ResourceConfigurationAPI<crate::function_instance::DomainManagedInstanceId>> {
        self.resource_configuration_client.clone()
    }
}

pub struct OrchestratorAPIServer {}

impl OrchestratorAPIServer {
    pub fn run(
        agent_api: Box<dyn crate::outer::orc::OrchestratorAPI + Send>,
        orchestrator_url: String,
        tls_config: Option<crate::grpc_impl::tls_config::TlsConfig>,
    ) -> futures::future::BoxFuture<'static, ()> {
        let mut agent_api = agent_api;
        let function_api = crate::grpc_impl::inner::function_instance::FunctionInstanceAPIServer::<crate::function_instance::DomainManagedInstanceId> {
            root_api: tokio::sync::Mutex::new(agent_api.function_instance_api()),
        };
        let resource_configuration_api = crate::grpc_impl::inner::resource_configuration::ResourceConfigurationServerHandler::<
            crate::function_instance::DomainManagedInstanceId,
        > {
            root_api: tokio::sync::Mutex::new(agent_api.resource_configuration_api()),
        };
        Box::pin(async move {
            let function_api = function_api;
            if let Ok((_proto, host, port)) = crate::util::parse_http_host(&orchestrator_url) {
                if let Ok(host) = format!("{}:{}", host, port).parse() {
                    log::info!("Start OrchestratorAPIServer GRPC Server at {}", orchestrator_url);

                    let mut server_builder = tonic::transport::Server::builder();

                    if let Some(tls_config) = tls_config {
                        match tls_config.create_server_tls_config() {
                            Ok(Some(config)) => {
                                log::info!("TLS enabled for GRPC server");
                                match server_builder.tls_config(config) {
                                    Ok(builder) => server_builder = builder,
                                    Err(e) => {
                                        log::error!("Failed to apply TLS config: {}", e);
                                        return;
                                    }
                                }
                            }
                            Ok(None) => {
                                log::info!("TLS disabled for GRPC server");
                            }
                            Err(e) => {
                                log::error!("Failed to create TLS config: {}", e);
                                return;
                            }
                        }
                    }

                    match server_builder
                        .add_service(
                            crate::grpc_impl::api::function_instance_server::FunctionInstanceServer::new(function_api)
                                .max_decoding_message_size(usize::MAX),
                        )
                        .add_service(
                            crate::grpc_impl::api::resource_configuration_server::ResourceConfigurationServer::new(resource_configuration_api)
                                .max_decoding_message_size(usize::MAX),
                        )
                        .serve(host)
                        .await
                    {
                        Ok(_) => {
                            log::debug!("Clean Exit");
                        }
                        Err(_) => {
                            log::error!("GRPC Server Failure");
                        }
                    }
                }
            }

            log::info!("Stop OrchestratorAPI GRPC Server");
        })
    }
}
