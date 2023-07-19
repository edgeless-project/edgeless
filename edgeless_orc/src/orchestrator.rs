use futures::{Future, SinkExt, StreamExt};

pub struct Orchestrator {
    sender: futures::channel::mpsc::UnboundedSender<OrchestratorRequest>,
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
    function_instance_client: Box<dyn edgeless_api::function_instance::FunctionInstanceAPI>,
}

impl edgeless_api::orc::OrchestratorAPI for OrchestratorClient {
    fn function_instance_api(&mut self) -> Box<dyn edgeless_api::function_instance::FunctionInstanceAPI> {
        self.function_instance_client.clone()
    }
}

#[derive(Clone)]
pub struct OrchestratorFunctionInstanceClient {
    sender: futures::channel::mpsc::UnboundedSender<OrchestratorRequest>,
}

impl OrchestratorFunctionInstanceClient {}

impl Orchestrator {
    pub fn new(node_settings: crate::EdgelessOrcSettings) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let main_task = Box::pin(async move {
            Self::main_task(receiver, node_settings).await;
        });

        (Orchestrator { sender }, main_task)
    }

    async fn main_task(receiver: futures::channel::mpsc::UnboundedReceiver<OrchestratorRequest>, orchestrator_settings: crate::EdgelessOrcSettings) {
        let mut clients = std::collections::HashMap::<uuid::Uuid, Box<dyn edgeless_api::agent::AgentAPI + Send>>::new();
        for node in &orchestrator_settings.nodes {
            clients.insert(
                node.node_id,
                Box::new(edgeless_api::grpc_impl::agent::AgentAPIClient::new(&node.agent_url).await),
            );
        }
        let mut receiver = receiver;
        let mut client = match clients.into_values().next() {
            Some(c) => c,
            None => {
                log::error!("Orchestrator without nodes. Exiting.");
                return;
            }
        };
        let mut fn_client = client.function_instance_api();
        while let Some(req) = receiver.next().await {
            match req {
                OrchestratorRequest::SPAWN(spawn_req, reply_channel) => {
                    log::debug!("Orchestrator Spawn {:?}", spawn_req);
                    let res = match fn_client.start_function_instance(spawn_req).await {
                        Ok(f_id) => Ok(f_id),
                        Err(err) => {
                            log::error!("Unhandled: {}", err);
                            Err(anyhow::anyhow!("Orchestrator->Node Spawn Request failed"))
                        }
                    };
                    if let Err(_) = reply_channel.send(res) {
                        log::error!("Orchestrator Reply Channel Error");
                    }
                }
                OrchestratorRequest::STOP(stop_function_id) => {
                    log::debug!("Orchestrator Stop {:?}", stop_function_id);
                    match fn_client.stop_function_instance(stop_function_id).await {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled: {}", err);
                        }
                    };
                }
                OrchestratorRequest::UPDATE(update) => {
                    log::debug!("Orchestrator Update {:?}", update);
                    match fn_client.update_function_instance_links(update).await {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled: {}", err);
                        }
                    };
                }
            }
        }
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::orc::OrchestratorAPI + Send> {
        Box::new(OrchestratorClient {
            function_instance_client: Box::new(OrchestratorFunctionInstanceClient { sender: self.sender.clone() }),
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
        if let Err(_) = self.sender.send(OrchestratorRequest::SPAWN(request, reply_sender)).await {
            return Err(anyhow::anyhow!("Orchestrator Channel Error"));
        }
        match reply_receiver.await {
            Ok(f_id) => f_id,
            Err(_) => Err(anyhow::anyhow!("Orchestrator Channel Error")),
        }
    }

    async fn stop_function_instance(&mut self, id: edgeless_api::function_instance::FunctionId) -> anyhow::Result<()> {
        match self.sender.send(OrchestratorRequest::STOP(id)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Orchestrator Channel Error")),
        }
    }

    async fn update_function_instance_links(&mut self, update: edgeless_api::function_instance::UpdateFunctionLinksRequest) -> anyhow::Result<()> {
        match self.sender.send(OrchestratorRequest::UPDATE(update)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Orchestrator Channel Error")),
        }
    }
}
