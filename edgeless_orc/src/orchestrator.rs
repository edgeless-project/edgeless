use futures::{Future, SinkExt, StreamExt};

pub struct Orchestrator {
    sender: futures::channel::mpsc::UnboundedSender<OrchestratorRequest>,
    node_settings: crate::EdgelessOrcSettings,
}

enum OrchestratorRequest {
    SPAWN(
        edgeless_api::function_instance::SpawnFunctionRequest,
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::function_instance::FunctionId>>,
    ),
    STOP(edgeless_api::function_instance::FunctionId),
    UPDATE(edgeless_api::function_instance::UpdateFunctionLinksRequest),
}

pub struct OrchestratorClient {
    function_instance_client: Option<Box<dyn edgeless_api::function_instance::FunctionInstanceAPI + Send>>,
}

impl edgeless_api::orc::OrchestratorAPI for OrchestratorClient {
    fn function_instance_api(&mut self) -> Box<dyn edgeless_api::function_instance::FunctionInstanceAPI + Send> {
        self.function_instance_client.take().unwrap()
    }
}

pub struct OrchestratorFunctionInstanceClient {
    sender: futures::channel::mpsc::UnboundedSender<OrchestratorRequest>,
}

impl OrchestratorFunctionInstanceClient {}

impl Orchestrator {
    pub fn new(node_settings: crate::EdgelessOrcSettings) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();

        let cloned_settings = node_settings.clone();
        let main_task = Box::pin(async move {
            let mut clients = std::collections::HashMap::<uuid::Uuid, Box<dyn edgeless_api::agent::AgentAPI + Send>>::new();
            for node in &cloned_settings.nodes {
                clients.insert(
                    node.node_id,
                    Box::new(edgeless_api::grpc_impl::agent::AgentAPIClient::new(&node.api_addr).await),
                );
            }
            Self::processing_loop(receiver, clients).await;
        });

        (Orchestrator { sender, node_settings }, main_task)
    }

    async fn processing_loop(
        receiver: futures::channel::mpsc::UnboundedReceiver<OrchestratorRequest>,
        clients: std::collections::HashMap<uuid::Uuid, Box<dyn edgeless_api::agent::AgentAPI + Send>>,
    ) {
        let mut receiver = receiver;
        let mut client = clients.into_values().next().unwrap();
        let mut fn_client = client.function_instance_api();
        while let Some(req) = receiver.next().await {
            match req {
                OrchestratorRequest::SPAWN(spawn_req, reply_channel) => {
                    log::debug!("Orchestrator Spawn {:?}", spawn_req);
                    reply_channel.send(fn_client.start_function_instance(spawn_req).await);
                }
                OrchestratorRequest::STOP(stop_function_id) => {
                    log::debug!("Orchestrator Stop {:?}", stop_function_id);
                    fn_client.stop_function_instance(stop_function_id).await;
                }
                OrchestratorRequest::UPDATE(update) => {
                    log::debug!("Orchestrator Update {:?}", update);
                    fn_client.update_function_instance_links(update).await;
                }
            }
        }
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::orc::OrchestratorAPI + Send> {
        Box::new(OrchestratorClient {
            function_instance_client: Some(Box::new(OrchestratorFunctionInstanceClient { sender: self.sender.clone() })),
        })
    }
}

#[async_trait::async_trait]
impl edgeless_api::function_instance::FunctionInstanceAPI for OrchestratorFunctionInstanceClient {
    async fn start_function_instance(
        &mut self,
        request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::function_instance::FunctionId> {
        let request = request;
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::function_instance::FunctionId>>();
        let _ = self.sender.send(OrchestratorRequest::SPAWN(request, reply_sender)).await;
        reply_receiver.await.unwrap()
    }
    async fn stop_function_instance(&mut self, id: edgeless_api::function_instance::FunctionId) -> anyhow::Result<()> {
        let _ = self.sender.send(OrchestratorRequest::STOP(id)).await;
        Ok(())
    }

    async fn update_function_instance_links(&mut self, update: edgeless_api::function_instance::UpdateFunctionLinksRequest) -> anyhow::Result<()> {
        let _ = self.sender.send(OrchestratorRequest::UPDATE(update)).await;
        Ok(())
    }
}
