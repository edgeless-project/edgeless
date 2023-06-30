pub struct OrchestratorAPIClient {
    function_instance_client: Option<Box<dyn crate::function_instance::FunctionInstanceAPI + Send>>,
}

impl OrchestratorAPIClient {
    pub async fn new(api_addr: &str) -> Self {
        Self {
            function_instance_client: Some(Box::new(
                crate::grpc_impl::function_instance::FunctionInstanceAPIClient::new(api_addr).await,
            )),
        }
    }
}

impl crate::orc::OrchestratorAPI for OrchestratorAPIClient {
    fn function_instance_api(&mut self) -> Box<dyn crate::function_instance::FunctionInstanceAPI + Send> {
        self.function_instance_client.take().unwrap()
    }
}

pub struct OrchestratorAPIServer {}

impl OrchestratorAPIServer {
    pub fn run(agent_api: Box<dyn crate::orc::OrchestratorAPI + Send>, listen_addr: String) -> futures::future::BoxFuture<'static, ()> {
        let mut agent_api = agent_api;
        let function_api = crate::grpc_impl::function_instance::FunctionInstanceAPIServer {
            root_api: tokio::sync::Mutex::new(agent_api.function_instance_api()),
        };
        Box::pin(async move {
            let function_api = function_api;
            let addr = listen_addr[7..].parse().unwrap();

            log::info!("Start OrchestratorAPI GRPC Server");

            tonic::transport::Server::builder()
                .add_service(crate::grpc_impl::api::function_instance_server::FunctionInstanceServer::new(function_api))
                .serve(addr)
                .await
                .unwrap();

            log::info!("Stop OrchestratorAPI GRPC Server");
        })
    }
}
