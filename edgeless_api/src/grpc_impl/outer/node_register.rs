// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub struct NodeRegisterAPIClient {
    node_registration_client: Box<dyn crate::node_registration::NodeRegistrationAPI>,
}

impl NodeRegisterAPIClient {
    pub async fn new(api_addr: String) -> Self {
        Self {
            node_registration_client: Box::new(crate::grpc_impl::node_registration::NodeRegistrationClient::new(api_addr)),
        }
    }
}

impl crate::outer::node_register::NodeRegisterAPI for NodeRegisterAPIClient {
    fn node_registration_api(&mut self) -> Box<dyn crate::node_registration::NodeRegistrationAPI> {
        self.node_registration_client.clone()
    }
}

pub struct NodeRegisterAPIServer {}

impl NodeRegisterAPIServer {
    pub fn run(
        agent_api: Box<dyn crate::outer::node_register::NodeRegisterAPI + Send>,
        node_register_url: String,
    ) -> futures::future::BoxFuture<'static, ()> {
        let mut agent_api = agent_api;
        let node_registration_api = crate::grpc_impl::node_registration::NodeRegistrationAPIService {
            node_registration_api: tokio::sync::Mutex::new(agent_api.node_registration_api()),
        };
        Box::pin(async move {
            let node_registration_api = node_registration_api;
            if let Ok((_proto, host, port)) = crate::util::parse_http_host(&node_register_url) {
                if let Ok(host) = format!("{}:{}", host, port).parse() {
                    log::info!("Start NodeRegisterAPIServer GRPC Server at {}", node_register_url);
                    match tonic::transport::Server::builder()
                        .add_service(
                            crate::grpc_impl::api::node_registration_server::NodeRegistrationServer::new(node_registration_api)
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

            log::info!("Stop NodeRegisterAPIServer GRPC Server");
        })
    }
}
