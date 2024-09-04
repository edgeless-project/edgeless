// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
pub struct AgentAPIClient {
    function_instance_client: Box<dyn crate::function_instance::FunctionInstanceAPI<edgeless_api_core::instance_id::InstanceId>>,
    node_management_client: Box<dyn crate::node_management::NodeManagementAPI>,
    resource_management_client: Box<dyn crate::resource_configuration::ResourceConfigurationAPI<edgeless_api_core::instance_id::InstanceId>>,
    link_instance_client: Box<dyn crate::link::LinkInstanceAPI>,
}

impl AgentAPIClient {
    pub async fn new(api_addr: &str) -> Self {
        Self {
            function_instance_client: Box::new(
                crate::grpc_impl::function_instance::FunctionInstanceAPIClient::<edgeless_api_core::instance_id::InstanceId>::new(api_addr, Some(1))
                    .await
                    .unwrap(),
            ),
            node_management_client: Box::new(
                crate::grpc_impl::node_management::NodeManagementClient::new(api_addr, Some(1))
                    .await
                    .unwrap(),
            ),
            resource_management_client: Box::new(crate::grpc_impl::resource_configuration::ResourceConfigurationClient::new(api_addr, Some(1)).await),
            link_instance_client: Box::new(crate::grpc_impl::link::LinkInstanceClient::new(api_addr, Some(1)).await)
        }
    }
}

impl crate::agent::AgentAPI for AgentAPIClient {
    fn function_instance_api(&mut self) -> Box<dyn crate::function_instance::FunctionInstanceAPI<edgeless_api_core::instance_id::InstanceId>> {
        self.function_instance_client.clone()
    }

    fn node_management_api(&mut self) -> Box<dyn crate::node_management::NodeManagementAPI> {
        self.node_management_client.clone()
    }

    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn crate::resource_configuration::ResourceConfigurationAPI<edgeless_api_core::instance_id::InstanceId>> {
        self.resource_management_client.clone()
    }

    fn link_instance_api(&mut self) -> Box<dyn crate::link::LinkInstanceAPI> {
        self.link_instance_client.clone()
    }
}

pub struct AgentAPIServer {}

impl AgentAPIServer {
    pub fn run(agent_api: Box<dyn crate::agent::AgentAPI + Send>, agent_url: String) -> futures::future::BoxFuture<'static, ()> {
        let mut agent_api = agent_api;
        let function_api = crate::grpc_impl::function_instance::FunctionInstanceAPIServer::<edgeless_api_core::instance_id::InstanceId> {
            root_api: tokio::sync::Mutex::new(agent_api.function_instance_api()),
        };
        let node_management_api = crate::grpc_impl::node_management::NodeManagementAPIService {
            node_management_api: tokio::sync::Mutex::new(agent_api.node_management_api()),
        };
        let resource_configuration_api =
            crate::grpc_impl::resource_configuration::ResourceConfigurationServerHandler::<edgeless_api_core::instance_id::InstanceId> {
                root_api: tokio::sync::Mutex::new(agent_api.resource_configuration_api()),
            };
        Box::pin(async move {
            let function_api = function_api;
            if let Ok((_proto, host, port)) = crate::util::parse_http_host(&agent_url) {
                if let Ok(host) = format!("{}:{}", host, port).parse() {
                    log::info!("Start AgentAPI GRPC Server at {}", agent_url);

                    match tonic::transport::Server::builder()
                        .add_service(
                            crate::grpc_impl::api::function_instance_server::FunctionInstanceServer::new(function_api)
                                .max_decoding_message_size(usize::MAX),
                        )
                        .add_service(
                            crate::grpc_impl::api::node_management_server::NodeManagementServer::new(node_management_api)
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

            log::info!("Stop AgentAPI GRPC Server");
        })
    }
}
