pub struct OrchestratorAPIClient {
    function_instance_client: Box<dyn crate::function_instance::FunctionInstanceOrcAPI>,
}

impl OrchestratorAPIClient {
    pub async fn new(api_addr: &str, retry_interval: Option<u64>) -> anyhow::Result<Self> {
        match crate::grpc_impl::function_instance::FunctionInstanceOrcAPIClient::new(api_addr, retry_interval).await {
            Ok(val) => Ok(Self {
                function_instance_client: Box::new(val),
            }),
            Err(err) => Err(err),
        }
    }
}

impl crate::orc::OrchestratorAPI for OrchestratorAPIClient {
    fn function_instance_api(&mut self) -> Box<dyn crate::function_instance::FunctionInstanceOrcAPI> {
        self.function_instance_client.clone()
    }
}

pub struct OrchestratorAPIServer {}

impl OrchestratorAPIServer {
    pub fn run(agent_api: Box<dyn crate::orc::OrchestratorAPI + Send>, orchestrator_url: String) -> futures::future::BoxFuture<'static, ()> {
        let mut agent_api = agent_api;
        let function_api = crate::grpc_impl::function_instance::FunctionInstanceOrcAPIServer {
            root_api: tokio::sync::Mutex::new(agent_api.function_instance_api()),
        };
        Box::pin(async move {
            let function_api = function_api;
            if let Ok((_proto, host, port)) = crate::util::parse_http_host(&orchestrator_url) {
                if let Ok(host) = format!("{}:{}", host, port).parse() {
                    log::info!("Start OrchestratorAPIServer GRPC Server at {}", orchestrator_url);
                    match tonic::transport::Server::builder()
                        .add_service(
                            crate::grpc_impl::api::function_instance_orc_server::FunctionInstanceOrcServer::new(function_api)
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
