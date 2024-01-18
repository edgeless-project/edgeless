// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
pub struct OrchestratorAPIClient {
    function_instance_client: Box<dyn crate::function_instance::FunctionInstanceAPI<crate::orc::DomainManagedInstanceId>>,
    node_registration_client: Box<dyn crate::node_registration::NodeRegistrationAPI>,
    resource_configuration_client: Box<dyn crate::resource_configuration::ResourceConfigurationAPI<crate::orc::DomainManagedInstanceId>>,
}

impl OrchestratorAPIClient {
    pub async fn new(api_addr: &str, retry_interval: Option<u64>) -> anyhow::Result<Self> {
        let function_instance_client = crate::grpc_impl::function_instance::FunctionInstanceAPIClient::new(api_addr, retry_interval).await;
        let node_registration_client = crate::grpc_impl::node_registration::NodeRegistrationClient::new(api_addr, retry_interval).await;
        let resource_configuration_client: Result<
            super::resource_configuration::ResourceConfigurationClient<crate::orc::DomainManagedInstanceId>,
            anyhow::Error,
        > = Ok(crate::grpc_impl::resource_configuration::ResourceConfigurationClient::new(api_addr, retry_interval).await);

        match (function_instance_client, node_registration_client, resource_configuration_client) {
            (Ok(function_instance_client), Ok(node_registration_client), Ok(resource_configuration_client)) => Ok(Self {
                function_instance_client: Box::new(function_instance_client),
                node_registration_client: Box::new(node_registration_client),
                resource_configuration_client: Box::new(resource_configuration_client),
            }),
            _ => Err(anyhow::anyhow!("One of the orc connections failed")),
        }
    }
}

impl crate::orc::OrchestratorAPI for OrchestratorAPIClient {
    fn function_instance_api(&mut self) -> Box<dyn crate::function_instance::FunctionInstanceAPI<crate::orc::DomainManagedInstanceId>> {
        self.function_instance_client.clone()
    }

    fn node_registration_api(&mut self) -> Box<dyn crate::node_registration::NodeRegistrationAPI> {
        self.node_registration_client.clone()
    }

    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn crate::resource_configuration::ResourceConfigurationAPI<crate::orc::DomainManagedInstanceId>> {
        self.resource_configuration_client.clone()
    }
}

pub struct OrchestratorAPIServer {}

impl OrchestratorAPIServer {
    pub fn run(agent_api: Box<dyn crate::orc::OrchestratorAPI + Send>, orchestrator_url: String) -> futures::future::BoxFuture<'static, ()> {
        let mut agent_api = agent_api;
        let function_api = crate::grpc_impl::function_instance::FunctionInstanceAPIServer::<crate::orc::DomainManagedInstanceId> {
            root_api: tokio::sync::Mutex::new(agent_api.function_instance_api()),
        };
        let node_registration_api = crate::grpc_impl::node_registration::NodeRegistrationAPIService {
            node_registration_api: tokio::sync::Mutex::new(agent_api.node_registration_api()),
        };
        let resource_configuration_api =
            crate::grpc_impl::resource_configuration::ResourceConfigurationServerHandler::<crate::orc::DomainManagedInstanceId> {
                root_api: tokio::sync::Mutex::new(agent_api.resource_configuration_api()),
            };
        Box::pin(async move {
            let function_api = function_api;
            if let Ok((_proto, host, port)) = crate::util::parse_http_host(&orchestrator_url) {
                if let Ok(host) = format!("{}:{}", host, port).parse() {
                    log::info!("Start OrchestratorAPIServer GRPC Server at {}", orchestrator_url);
                    match tonic::transport::Server::builder()
                        .add_service(
                            crate::grpc_impl::api::function_instance_server::FunctionInstanceServer::new(function_api)
                                .max_decoding_message_size(usize::MAX),
                        )
                        .add_service(
                            crate::grpc_impl::api::node_registration_server::NodeRegistrationServer::new(node_registration_api)
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
