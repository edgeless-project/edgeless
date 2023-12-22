use edgeless_api::function_instance::UpdatePeersRequest;
use edgeless_dataplane::core::EdgelessDataplanePeerSettings;
use futures::{Future, SinkExt, StreamExt};

use crate::runner_api;

enum AgentRequest {
    SPAWN(edgeless_api::function_instance::SpawnFunctionRequest),
    STOP(edgeless_api::function_instance::InstanceId),
    UPDATELINKS(edgeless_api::function_instance::UpdateFunctionLinksRequest),
    UPDATEPEERS(edgeless_api::function_instance::UpdatePeersRequest),
}

pub struct Agent {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
    node_settings: crate::EdgelessNodeSettings,
}

impl Agent {
    pub fn new(
        runner: Box<dyn runner_api::RunnerAPI + Send>,
        node_settings: crate::EdgelessNodeSettings,
        data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
    ) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();

        let main_task = Box::pin(async move {
            Self::main_task(receiver, runner, data_plane_provider).await;
        });

        (Agent { sender, node_settings }, main_task)
    }

    async fn main_task(
        receiver: futures::channel::mpsc::UnboundedReceiver<AgentRequest>,
        runner: Box<dyn runner_api::RunnerAPI + Send>,
        data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
    ) {
        let mut receiver = std::pin::pin!(receiver);
        let mut runner = runner;
        let mut data_plane_provider = data_plane_provider;
        log::info!("Starting Edgeless Agent");
        while let Some(req) = receiver.next().await {
            match req {
                AgentRequest::SPAWN(spawn_req) => {
                    log::debug!("Agent Spawn {:?}", spawn_req);
                    match runner.start(spawn_req).await {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled Start Error: {}", err);
                        }
                    }
                }
                AgentRequest::STOP(stop_function_id) => {
                    log::debug!("Agent Stop {:?}", stop_function_id);
                    match runner.stop(stop_function_id).await {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled Stop Error: {}", err);
                        }
                    }
                }
                AgentRequest::UPDATELINKS(update) => {
                    log::debug!("Agent UpdatePeers {:?}", update);
                    match runner.update_links(update).await {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled UpdateLinks Error: {}", err);
                        }
                    }
                }
                AgentRequest::UPDATEPEERS(request) => {
                    log::debug!("Agent UpdatePeers {:?}", request);
                    match request {
                        UpdatePeersRequest::Add(node_id, invocation_url) => {
                            data_plane_provider
                                .add_peer(EdgelessDataplanePeerSettings { node_id, invocation_url })
                                .await
                        }
                        UpdatePeersRequest::Del(node_id) => data_plane_provider.del_peer(node_id).await,
                        UpdatePeersRequest::Clear => panic!("UpdatePeersRequest::Clear not implemented"),
                    };
                }
            }
        }
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::agent::AgentAPI + Send> {
        Box::new(AgentClient {
            function_instance_client: Box::new(FunctionInstanceNodeClient {
                sender: self.sender.clone(),
                node_id: self.node_settings.node_id.clone(),
            }),
        })
    }
}

#[derive(Clone)]
pub struct FunctionInstanceNodeClient {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
    node_id: uuid::Uuid,
}

#[derive(Clone)]
pub struct AgentClient {
    function_instance_client: Box<dyn edgeless_api::function_instance::FunctionInstanceNodeAPI>,
}

#[async_trait::async_trait]
impl edgeless_api::function_instance::FunctionInstanceNodeAPI for FunctionInstanceNodeClient {
    async fn start(
        &mut self,
        request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse> {
        let mut request = request;
        let f_id = match request.instance_id.clone() {
            Some(id) => id,
            None => {
                let new_id = edgeless_api::function_instance::InstanceId::new(self.node_id);
                request.instance_id = Some(new_id.clone());
                new_id
            }
        };
        match self.sender.send(AgentRequest::SPAWN(request)).await {
            Ok(_) => Ok(edgeless_api::common::StartComponentResponse::InstanceId(f_id)),
            Err(err) => Err(anyhow::anyhow!(
                "Agent channel error when creating a function instance: {}",
                err.to_string()
            )),
        }
    }
    async fn stop(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        match self.sender.send(AgentRequest::STOP(id)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Agent channel error when stopping a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn update_links(&mut self, update: edgeless_api::function_instance::UpdateFunctionLinksRequest) -> anyhow::Result<()> {
        match self.sender.send(AgentRequest::UPDATELINKS(update)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Agent channel error when updating the links of a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn update_peers(&mut self, request: edgeless_api::function_instance::UpdatePeersRequest) -> anyhow::Result<()> {
        match self.sender.send(AgentRequest::UPDATEPEERS(request)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Agent channel error when updating the peers of a node: {}",
                err.to_string()
            )),
        }
    }

    async fn keep_alive(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl edgeless_api::agent::AgentAPI for AgentClient {
    fn function_instance_api(&mut self) -> Box<dyn edgeless_api::function_instance::FunctionInstanceNodeAPI> {
        self.function_instance_client.clone()
    }
}
