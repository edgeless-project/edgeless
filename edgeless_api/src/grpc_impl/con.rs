pub struct ControllerAPIClient {
    workflow_instance_client: Option<Box<dyn crate::workflow_instance::WorkflowInstanceAPI + Send>>,
}

impl ControllerAPIClient {
    pub async fn new(api_addr: &str) -> Self {
        Self {
            workflow_instance_client: Some(Box::new(
                crate::grpc_impl::workflow_instance::WorkflowInstanceAPIClient::new(api_addr).await,
            )),
        }
    }
}

impl crate::con::ControllerAPI for ControllerAPIClient {
    fn workflow_instance_api(&mut self) -> Box<dyn crate::workflow_instance::WorkflowInstanceAPI + Send> {
        self.workflow_instance_client.take().unwrap()
    }
}

pub struct WorkflowInstanceAPIServer {}

impl WorkflowInstanceAPIServer {
    pub fn run(controller_api: Box<dyn crate::con::ControllerAPI + Send>, listen_addr: String) -> futures::future::BoxFuture<'static, ()> {
        let mut controller_api = controller_api;
        let workflow_api = crate::grpc_impl::workflow_instance::WorkflowInstanceAPIServer {
            root_api: tokio::sync::Mutex::new(controller_api.workflow_instance_api()),
        };
        Box::pin(async move {
            let workflow_api = workflow_api;
            let addr = listen_addr[7..].parse().unwrap();

            log::info!("Start ControllerAPI GRPC Server");

            tonic::transport::Server::builder()
                .add_service(crate::grpc_impl::api::workflow_instance_server::WorkflowInstanceServer::new(workflow_api))
                .serve(addr)
                .await
                .unwrap();

            log::info!("Stop ControllerAPI GRPC Server");
        })
    }
}
