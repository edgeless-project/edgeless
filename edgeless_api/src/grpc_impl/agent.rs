use super::function_instance::FunctionInstanceNodeAPIServer;

pub struct AgentAPIClient {
    function_instance_client: Box<dyn crate::function_instance::FunctionInstanceNodeAPI>,
}

impl AgentAPIClient {
    pub async fn new(api_addr: &str) -> Self {
        Self {
            function_instance_client: Box::new(
                crate::grpc_impl::function_instance::FunctionInstanceNodeAPIClient::new(api_addr, Some(1))
                    .await
                    .unwrap(),
            ),
        }
    }
}

impl crate::agent::AgentAPI for AgentAPIClient {
    fn function_instance_api(&mut self) -> Box<dyn crate::function_instance::FunctionInstanceNodeAPI> {
        self.function_instance_client.clone()
    }
}

pub struct AgentAPIServer {}

impl AgentAPIServer {
    pub fn run(agent_api: Box<dyn crate::agent::AgentAPI + Send>, agent_url: String) -> futures::future::BoxFuture<'static, ()> {
        let mut agent_api = agent_api;
        let function_api = FunctionInstanceNodeAPIServer {
            root_api: tokio::sync::Mutex::new(agent_api.function_instance_api()),
        };
        Box::pin(async move {
            let function_api = function_api;
            if let Ok((_proto, host, port)) = crate::util::parse_http_host(&agent_url) {
                if let Ok(host) = format!("{}:{}", host, port).parse() {
                    log::info!("Start AgentAPI GRPC Server at {}", agent_url);

                    match tonic::transport::Server::builder()
                        .add_service(
                            crate::grpc_impl::api::function_instance_node_server::FunctionInstanceNodeServer::new(function_api)
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
