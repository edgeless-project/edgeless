use super::function_instance::FunctionInstanceAPIServer;

pub struct AgentAPIClient {
    function_instance_client: Option<Box<dyn crate::function_instance::FunctionInstanceAPI + Send>>,
}

impl AgentAPIClient {
    pub async fn new(api_addr: &str) -> Self {
        Self {
            function_instance_client: Some(Box::new(
                crate::grpc_impl::function_instance::FunctionInstanceAPIClient::new(api_addr).await,
            )),
        }
    }
}

impl crate::agent::AgentAPI for AgentAPIClient {
    fn function_instance_api(&mut self) -> Box<dyn crate::function_instance::FunctionInstanceAPI + Send> {
        self.function_instance_client.take().unwrap()
    }
}

pub struct AgentAPIServer {}

impl AgentAPIServer {
    pub fn run(agent_api: Box<dyn crate::agent::AgentAPI + Send>, listen_addr: String) -> futures::future::BoxFuture<'static, ()> {
        let mut agent_api = agent_api;
        let function_api = FunctionInstanceAPIServer {
            root_api: tokio::sync::Mutex::new(agent_api.function_instance_api()),
        };
        Box::pin(async move {
            let function_api = function_api;
            let addr = listen_addr[7..].parse().unwrap();

            log::info!("Start AgentAPI GRPC Server");

            tonic::transport::Server::builder()
                .add_service(crate::grpc_impl::api::function_instance_server::FunctionInstanceServer::new(function_api))
                .serve(addr)
                .await
                .unwrap();

            log::info!("Stop AgentAPI GRPC Server");
        })
    }
}
