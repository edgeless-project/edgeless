// SPDX-FileCopyrightText: © 2023 TUM
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
pub struct ControllerAPIClient {
    workflow_instance_client: Box<dyn crate::workflow_instance::WorkflowInstanceAPI>,
}

impl ControllerAPIClient {
    pub async fn new(api_addr: &str) -> Self {
        Self {
            workflow_instance_client: Box::new(crate::grpc_impl::workflow_instance::WorkflowInstanceAPIClient::new(api_addr).await),
        }
    }
}

impl crate::controller::ControllerAPI for ControllerAPIClient {
    fn workflow_instance_api(&mut self) -> Box<dyn crate::workflow_instance::WorkflowInstanceAPI> {
        self.workflow_instance_client.clone()
    }
}

pub struct WorkflowInstanceAPIServer {}

impl WorkflowInstanceAPIServer {
    pub fn run(controller_api: Box<dyn crate::controller::ControllerAPI + Send>, controller_url: String) -> futures::future::BoxFuture<'static, ()> {
        let mut controller_api = controller_api;
        let workflow_api = crate::grpc_impl::workflow_instance::WorkflowInstanceAPIServer {
            root_api: tokio::sync::Mutex::new(controller_api.workflow_instance_api()),
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
