// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
pub struct ControllerAPIClient {
    workflow_instance_client: Box<dyn crate::workflow_instance::WorkflowInstanceAPI>,
    node_registration_client: Box<dyn crate::node_registration::NodeRegistrationAPI>,
}

impl ControllerAPIClient {
    pub async fn new(api_addr: &str) -> Self {
        Self {
            workflow_instance_client: Box::new(crate::grpc_impl::workflow_instance::WorkflowInstanceAPIClient::new(api_addr.clone()).await),
            node_registration_client: Box::new(
                crate::grpc_impl::node_registration::NodeRegistrationClient::new(api_addr, Some(5))
                    .await
                    .unwrap(),
            ),
        }
    }
}

impl crate::controller::ControllerAPI for ControllerAPIClient {
    fn workflow_instance_api(&mut self) -> Box<dyn crate::workflow_instance::WorkflowInstanceAPI> {
        self.workflow_instance_client.clone()
    }

    fn node_registration_api(&mut self) -> Box<dyn crate::node_registration::NodeRegistrationAPI> {
        self.node_registration_client.clone()
    }
}

pub struct WorkflowInstanceAPIServer {}

impl WorkflowInstanceAPIServer {
    pub fn run(controller_api: Box<dyn crate::controller::ControllerAPI + Send>, controller_url: String) -> futures::future::BoxFuture<'static, ()> {
        let mut controller_api = controller_api;
        let workflow_api = crate::grpc_impl::workflow_instance::WorkflowInstanceAPIServer {
            root_api: tokio::sync::Mutex::new(controller_api.workflow_instance_api()),
        };
        let node_registration_api = crate::grpc_impl::node_registration::NodeRegistrationAPIService {
            node_registration_api: tokio::sync::Mutex::new(controller_api.node_registration_api()),
        };
        Box::pin(async move {
            let workflow_api = workflow_api;
            if let Ok((_proto, host, port)) = crate::util::parse_http_host(&controller_url) {
                if let Ok(host) = format!("{}:{}", host, port).parse() {
                    log::info!("Start ControllerAPI GRPC Server at {}", controller_url);

                    match tonic::transport::Server::builder()
                        .add_service(
                            crate::grpc_impl::api::workflow_instance_server::WorkflowInstanceServer::new(workflow_api)
                                .max_decoding_message_size(usize::MAX),
                        )
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

            log::info!("Stop ControllerAPI GRPC Server");
        })
    }
}
