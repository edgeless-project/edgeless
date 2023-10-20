use futures::{Future, SinkExt, StreamExt};

pub struct Orchestrator {
    sender: futures::channel::mpsc::UnboundedSender<OrchestratorRequest>,
}

enum OrchestratorRequest {
    // TODO: why does SpawnFunctionRequest already container instance_id?
    // shouldn't this be decided by the orchestrator?
    SPAWN(
        // contains: instance_id (node + function id), code, output_callbacks?
        edgeless_api::function_instance::SpawnFunctionRequest,
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::function_instance::SpawnFunctionResponse>>,
    ),
    STOP(edgeless_api::function_instance::InstanceId),
    UPDATE(edgeless_api::function_instance::UpdateFunctionLinksRequest),
}

// TODO: what is the role of OrchestratorClient?
pub struct OrchestratorClient {
    function_instance_client: Box<dyn edgeless_api::function_instance::FunctionInstanceAPI>,
}

impl edgeless_api::orc::OrchestratorAPI for OrchestratorClient {
    // TODO: what is the role of this?
    fn function_instance_api(&mut self) -> Box<dyn edgeless_api::function_instance::FunctionInstanceAPI> {
        self.function_instance_client.clone()
    }
}

#[derive(Clone)]
// TODO: what is the role of this component?
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
        // Goes through the list of all worker nodes in this orchestration
        // domain and creates AgentAPIClient objects for them
        for node in &orchestrator_settings.nodes {
            clients.insert(
                node.node_id,
                Box::new(edgeless_api::grpc_impl::agent::AgentAPIClient::new(&node.agent_url).await),
            );
        }
        // Receiver TODO: explain what it does
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
                    // TODO: here we should perform the selection of the client to which we
                    // will send the spawn request

                    // TODO: my code will go here

                    log::debug!("Orchestrator Spawn {:?}", spawn_req);
                    let res = match fn_client.start(spawn_req).await {
                        Ok(res) => Ok(res),
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
                    // TODO: orchestrator maintains a list map where
                    // function_ids are mapped to the worker node
                    log::debug!("Orchestrator Stop {:?}", stop_function_id);
                    match fn_client.stop(stop_function_id).await {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled: {}", err);
                        }
                    };
                }
                OrchestratorRequest::UPDATE(update) => {
                    // TODO: orchestrator maintains a list map where
                    // function_ids are mapped to the client id
                    log::debug!("Orchestrator Update {:?}", update);
                    match fn_client.update_links(update).await {
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
// It sends events on the sender?
impl edgeless_api::function_instance::FunctionInstanceAPI for OrchestratorFunctionInstanceClient {
    async fn start(
        &mut self,
        request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::function_instance::SpawnFunctionResponse> {
        let request = request;
        let (reply_sender, reply_receiver) =
            tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::function_instance::SpawnFunctionResponse>>();
        if let Err(err) = self.sender.send(OrchestratorRequest::SPAWN(request, reply_sender)).await {
            return Err(anyhow::anyhow!(
                "Orchestrator channel error when creating a function instance: {}",
                err.to_string()
            ));
        }
        match reply_receiver.await {
            Ok(f_id) => f_id,
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when creating a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn stop(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        match self.sender.send(OrchestratorRequest::STOP(id)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when stopping a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn update_links(&mut self, update: edgeless_api::function_instance::UpdateFunctionLinksRequest) -> anyhow::Result<()> {
        match self.sender.send(OrchestratorRequest::UPDATE(update)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when updating the links of a function instance: {}",
                err.to_string()
            )),
        }
    }
}
