use edgeless_api::function_instance::SpawnFunctionResponse;
use futures::{Future, SinkExt, StreamExt};
use std::collections::HashMap;

pub struct Orchestrator {
    sender: futures::channel::mpsc::UnboundedSender<OrchestratorRequest>,
}

enum OrchestratorRequest {
    SPAWN(
        edgeless_api::function_instance::SpawnFunctionRequest,
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::function_instance::SpawnFunctionResponse>>,
    ),
    STOP(edgeless_api::function_instance::InstanceId),
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
        log::info!("Main_task started!");
        let mut clients = HashMap::<uuid::Uuid, Box<dyn edgeless_api::agent::AgentAPI + Send>>::new();
        // Goes through the list of all worker nodes in this orchestration
        // domain and creates AgentAPIClient objects for them that will be used
        // to control them
        for node in &orchestrator_settings.nodes {
            clients.insert(
                node.node_id,
                Box::new(edgeless_api::grpc_impl::agent::AgentAPIClient::new(&node.agent_url).await),
            );
        }
        let mut receiver = receiver;

        // Initializes the OrchestrationLogic
        let mut orchestration_logic = crate::orchestration_logic::OrchestrationLogic::new(
            Some(orchestrator_settings.orchestration_strategy),
            clients.keys().cloned().collect(),
        );

        // Main loop that reacts to events on the receiver channel
        while let Some(req) = receiver.next().await {
            match req {
                OrchestratorRequest::SPAWN(spawn_req, reply_channel) => {
                    // Orchestration step: select the node to spawn this
                    // function instance by using the orchestration logic.
                    // Orchestration strategy can also be changed during
                    // runtime.
                    let selected_node_id = match orchestration_logic.next() {
                        Some(u) => u,
                        None => {
                            log::error!("Could not select the next node. Either no nodes are specified or an error occured. Exiting.");
                            return;
                        }
                    };

                    let mut fn_client = match clients.get_mut(&selected_node_id) {
                        Some(c) => c,
                        None => {
                            log::error!("Invalid node selected by the orchestration logic. Exiting.");
                            return;
                        }
                    }
                    .function_instance_api();
                    log::debug!("Orchestrator Spawn {:?} at worker node with node_id {:?}", spawn_req, selected_node_id);

                    // Finally try to spawn the function instance on the
                    // selected client
                    let res = match fn_client.start(spawn_req).await {
                        Ok(res) => match res {
                            SpawnFunctionResponse::ResponseError(err) => Err(anyhow::anyhow!("Orchestrator->Node Spawn Request failed: {}", &err)),
                            SpawnFunctionResponse::InstanceId(id) => {
                                log::info!("Spawned at: {:?}", &id);
                                Ok(res)
                            }
                        },
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
                    let mut fn_client = match clients.get_mut(&stop_function_id.node_id) {
                        Some(c) => c,
                        None => {
                            log::error!("This orchestrator does not manage the node where this function instance {:?} is located! Please note that support for multiple orchestrators is not implemented yet!", stop_function_id);
                            return;
                        }
                    }.function_instance_api();

                    match fn_client.stop(stop_function_id).await {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled: {}", err);
                        }
                    };
                }
                OrchestratorRequest::UPDATE(update) => {
                    log::debug!("Orchestrator Update {:?}", update);
                    if let Some(instance_id) = update.clone().instance_id {
                        let mut fn_client = match clients.get_mut(&instance_id.node_id) {
                            Some(c) => c,
                            None => {
                                log::error!("This orchestrator does not manage the node where this function instance {:?} is located! Please note that support for multiple orchestrators is not implemented yet!", instance_id);
                                return;
                            }
                        }.function_instance_api();

                        match fn_client.update_links(update).await {
                            Ok(_) => {}
                            Err(err) => {
                                log::error!("Unhandled: {}", err);
                            }
                        };
                    } else {
                        log::error!("A request to an orchestrator to update links must contain a valid InstanceId!");
                    }
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
    async fn start(
        &mut self,
        request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::function_instance::SpawnFunctionResponse> {
        let request = request;
        println!("{:?}", request.instance_id);
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
