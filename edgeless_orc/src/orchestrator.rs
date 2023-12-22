use edgeless_api::common::StartComponentResponse;
use edgeless_api::function_instance::{UpdateNodeRequest, UpdatePeersRequest};
use futures::{Future, SinkExt, StreamExt};
use std::collections::{HashMap, HashSet};

pub struct Orchestrator {
    sender: futures::channel::mpsc::UnboundedSender<OrchestratorRequest>,
}

enum OrchestratorRequest {
    STARTFUNCTION(
        edgeless_api::function_instance::SpawnFunctionRequest,
        tokio::sync::oneshot::Sender<anyhow::Result<StartComponentResponse>>,
    ),
    STOPFUNCTION(edgeless_api::function_instance::InstanceId),
    STARTRESOURCE(
        edgeless_api::workflow_instance::WorkflowResource,
        tokio::sync::oneshot::Sender<anyhow::Result<StartComponentResponse>>,
    ),
    STOPRESOURCE(edgeless_api::function_instance::InstanceId),
    UPDATELINKS(edgeless_api::function_instance::UpdateFunctionLinksRequest),
    UPDATENODE(
        edgeless_api::function_instance::UpdateNodeRequest,
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::function_instance::UpdateNodeResponse>>,
    ),
    KEEPALIVE(),
}

pub struct OrchestratorClient {
    function_instance_client: Box<dyn edgeless_api::function_instance::FunctionInstanceOrcAPI>,
}

impl edgeless_api::orc::OrchestratorAPI for OrchestratorClient {
    fn function_instance_api(&mut self) -> Box<dyn edgeless_api::function_instance::FunctionInstanceOrcAPI> {
        self.function_instance_client.clone()
    }
}

#[derive(Clone)]
pub struct OrchestratorFunctionInstanceOrcClient {
    sender: futures::channel::mpsc::UnboundedSender<OrchestratorRequest>,
}

impl OrchestratorFunctionInstanceOrcClient {}

pub struct ClientDesc {
    agent_url: String,
    invocation_url: String,
    api: Box<dyn edgeless_api::agent::AgentAPI + Send>,
}

impl Orchestrator {
    pub fn new(node_settings: crate::EdgelessOrcSettings) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let main_task = Box::pin(async move {
            Self::main_task(receiver, node_settings).await;
        });

        (Orchestrator { sender }, main_task)
    }

    pub async fn keep_alive(&mut self) {
        let _ = self.sender.send(OrchestratorRequest::KEEPALIVE()).await;
    }

    async fn main_task(receiver: futures::channel::mpsc::UnboundedReceiver<OrchestratorRequest>, orchestrator_settings: crate::EdgelessOrcSettings) {
        let mut clients = HashMap::<uuid::Uuid, ClientDesc>::new();
        let mut receiver = receiver;
        let mut orchestration_logic = crate::orchestration_logic::OrchestrationLogic::new(orchestrator_settings.orchestration_strategy);

        // Main loop that reacts to events on the receiver channel
        while let Some(req) = receiver.next().await {
            match req {
                OrchestratorRequest::STARTFUNCTION(spawn_req, reply_channel) => {
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
                    .api
                    .function_instance_api();
                    log::debug!("Orchestrator Spawn {:?} at worker node with node_id {:?}", spawn_req, selected_node_id);

                    // Finally try to spawn the function instance on the
                    // selected client
                    let res = match fn_client.start(spawn_req).await {
                        Ok(res) => match res {
                            StartComponentResponse::ResponseError(err) => Err(anyhow::anyhow!("Orchestrator->Node Spawn Request failed: {}", &err)),
                            StartComponentResponse::InstanceId(id) => {
                                log::info!("Spawned at: {:?}", &id);
                                Ok(res)
                            }
                        },
                        Err(err) => {
                            log::error!("Unhandled: {}", err);
                            Err(anyhow::anyhow!("Orchestrator->Node Spawn Request failed"))
                        }
                    };
                    if let Err(err) = reply_channel.send(res) {
                        log::error!("Orchestrator channel error in SPAWN: {:?}", err);
                    }
                }
                OrchestratorRequest::STOPFUNCTION(stop_function_id) => {
                    log::debug!("Orchestrator StopFunction {:?}", stop_function_id);
                    let mut fn_client = match clients.get_mut(&stop_function_id.node_id) {
                        Some(c) => c,
                        None => {
                            log::error!("This orchestrator does not manage the node where this function instance {:?} is located! Please note that support for multiple orchestrators is not implemented yet!", stop_function_id);
                            return;
                        }
                    }.api.function_instance_api();

                    match fn_client.stop(stop_function_id).await {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled: {}", err);
                        }
                    };
                }
                OrchestratorRequest::STARTRESOURCE(spawn_req, reply_channel) => {
                    // XXX Issue#60
                }
                OrchestratorRequest::STOPRESOURCE(stop_function_id) => {
                    log::debug!("Orchestrator StopResource {:?}", stop_function_id);
                    // XXX Issue#60
                }
                OrchestratorRequest::UPDATELINKS(update) => {
                    log::debug!("Orchestrator Update {:?}", update);
                    if let Some(instance_id) = update.clone().instance_id {
                        let mut fn_client = match clients.get_mut(&instance_id.node_id) {
                            Some(c) => c,
                            None => {
                                log::error!("This orchestrator does not manage the node where this function instance {:?} is located! Please note that support for multiple orchestrators is not implemented yet!", instance_id);
                                return;
                            }
                        }.api.function_instance_api();

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
                OrchestratorRequest::UPDATENODE(request, reply_channel) => {
                    // Update the map of clients and, at the same time, prepare
                    // the UpdatePeersRequest message to be sent to all the
                    // clients to notify that a new node exists (Register) or
                    // that an existing node left the system (Deregister).
                    let mut this_node_id = None;
                    let msg = match request {
                        UpdateNodeRequest::Registration(node_id, agent_url, invocation_url) => {
                            let mut dup_entry = false;
                            if let Some(client_desc) = clients.get(&node_id) {
                                if client_desc.agent_url == agent_url && client_desc.invocation_url == invocation_url {
                                    dup_entry = true;
                                }
                            }
                            if dup_entry {
                                // A client with same node_id, agent_url, and
                                // invocation_url already exists.
                                None
                            } else {
                                this_node_id = Some(node_id.clone());
                                clients.insert(
                                    node_id,
                                    ClientDesc {
                                        agent_url: agent_url.clone(),
                                        invocation_url: invocation_url.clone(),
                                        api: Box::new(edgeless_api::grpc_impl::agent::AgentAPIClient::new(&agent_url).await),
                                    },
                                );
                                Some(UpdatePeersRequest::Add(node_id, invocation_url))
                            }
                        }
                        UpdateNodeRequest::Deregistration(node_id) => {
                            if let None = clients.get(&node_id) {
                                // There is no client with that node_id
                                None
                            } else {
                                clients.remove(&node_id);
                                Some(UpdatePeersRequest::Del(node_id))
                            }
                        }
                    };

                    // If no operation was done (either a new node was already
                    // present with same agent/invocation URLs or a deregistering
                    // node did not exist) we accept the command.
                    let mut response = edgeless_api::function_instance::UpdateNodeResponse::Accepted;

                    if let Some(msg) = msg {
                        // Update the orchestration logic with the new set of nodes.
                        orchestration_logic.update_nodes(clients.keys().cloned().collect());

                        // Update all the peers (including the node, unless it
                        // was a deregister operation).
                        let mut num_failures: u32 = 0;
                        for (_node_id, client) in clients.iter_mut() {
                            if let Err(_) = client.api.function_instance_api().update_peers(msg.clone()).await {
                                num_failures += 1;
                            }
                        }

                        // Only with registration, we also update the new node
                        // by adding as peers all the existing nodes.
                        if let Some(this_node_id) = this_node_id {
                            let mut new_node_client = clients.get_mut(&this_node_id).unwrap().api.function_instance_api();
                            for (other_node_id, client_desc) in clients.iter_mut() {
                                if other_node_id.eq(&this_node_id) {
                                    continue;
                                }
                                if let Err(_) = new_node_client
                                    .update_peers(UpdatePeersRequest::Add(*other_node_id, client_desc.invocation_url.clone()))
                                    .await
                                {
                                    num_failures += 1;
                                }
                            }
                        }

                        response = match num_failures {
                            0 => edgeless_api::function_instance::UpdateNodeResponse::Accepted,
                            _ => edgeless_api::function_instance::UpdateNodeResponse::ResponseError(edgeless_api::common::ResponseError {
                                summary: "UpdatePeers() failed on some node when updating a node".to_string(),
                                detail: None,
                            }),
                        };
                    }

                    if let Err(err) = reply_channel.send(Ok(response)) {
                        log::error!("Orchestrator channel error in UPDATENODE: {:?}", err);
                    }
                }
                OrchestratorRequest::KEEPALIVE() => {
                    log::debug!("keep alive");

                    // First check if there nodes that must be disconnected,
                    // since they fail to reply to a keep-alive.
                    let mut to_be_disconnected = HashSet::new();
                    for (node_id, client_desc) in &mut clients {
                        if let Err(_) = client_desc.api.function_instance_api().keep_alive().await {
                            to_be_disconnected.insert(*node_id);
                        }
                    }

                    // Second, remove all those nodes from the map of clients.
                    for node_id in to_be_disconnected.iter() {
                        log::info!("disconnect node not replying to keep alive: {}", &node_id);
                        let val = clients.remove(&node_id);
                        assert!(val.is_some());
                    }

                    // Finally, update the peers of (still alive) nodes by
                    // deleting the missing-in-action peers.
                    for removed_node_id in to_be_disconnected {
                        for (_, client_desc) in clients.iter_mut() {
                            match client_desc
                                .api
                                .function_instance_api()
                                .update_peers(UpdatePeersRequest::Del(removed_node_id))
                                .await
                            {
                                Ok(_) => {}
                                Err(err) => {
                                    log::error!("Unhandled: {}", err);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::orc::OrchestratorAPI + Send> {
        Box::new(OrchestratorClient {
            function_instance_client: Box::new(OrchestratorFunctionInstanceOrcClient { sender: self.sender.clone() }),
        })
    }
}

#[async_trait::async_trait]
impl edgeless_api::function_instance::FunctionInstanceOrcAPI for OrchestratorFunctionInstanceOrcClient {
    async fn start_function(&mut self, request: edgeless_api::function_instance::SpawnFunctionRequest) -> anyhow::Result<StartComponentResponse> {
        log::debug!("FunctionInstance::StartFunction() {:?}", request);
        let request = request;
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<StartComponentResponse>>();
        if let Err(err) = self.sender.send(OrchestratorRequest::STARTFUNCTION(request, reply_sender)).await {
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

    async fn stop_function(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        log::debug!("FunctionInstance::StopFunction() {:?}", id);
        match self.sender.send(OrchestratorRequest::STOPFUNCTION(id)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when stopping a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn start_resource(
        &mut self,
        request: edgeless_api::workflow_instance::WorkflowResource,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse> {
        log::debug!("FunctionInstance::StartResource() {:?}", request);
        let request = request;
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::common::StartComponentResponse>>();
        if let Err(err) = self.sender.send(OrchestratorRequest::STARTRESOURCE(request, reply_sender)).await {
            return Err(anyhow::anyhow!(
                "Orchestrator channel error when starting a resource: {}",
                err.to_string()
            ));
        }
        match reply_receiver.await {
            Ok(f_id) => f_id,
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when starting a resource: {}",
                err.to_string()
            )),
        }
    }

    async fn stop_resource(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        log::debug!("FunctionInstance::StopResource() {:?}", id);
        match self.sender.send(OrchestratorRequest::STOPRESOURCE(id)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when stopping a resource: {}",
                err.to_string()
            )),
        }
    }

    async fn update_links(&mut self, update: edgeless_api::function_instance::UpdateFunctionLinksRequest) -> anyhow::Result<()> {
        log::debug!("FunctionInstance::UpdateLinks() {:?}", update);
        match self.sender.send(OrchestratorRequest::UPDATELINKS(update)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when updating the links of a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn update_node(
        &mut self,
        request: edgeless_api::function_instance::UpdateNodeRequest,
    ) -> anyhow::Result<edgeless_api::function_instance::UpdateNodeResponse> {
        log::debug!("FunctionInstance::UpdateNode() {:?}", request);
        let request = request;
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::function_instance::UpdateNodeResponse>>();
        if let Err(err) = self.sender.send(OrchestratorRequest::UPDATENODE(request, reply_sender)).await {
            return Err(anyhow::anyhow!("Orchestrator channel error when updating a node: {}", err.to_string()));
        }
        match reply_receiver.await {
            Ok(res) => res,
            Err(err) => Err(anyhow::anyhow!("Orchestrator channel error  when updating a node: {}", err.to_string())),
        }
    }
}
