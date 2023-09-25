use futures::{Future, SinkExt, StreamExt};

use crate::runner_api;

enum AgentRequest {
    SPAWN(edgeless_api::function_instance::SpawnFunctionRequest),
    STOP(edgeless_api::function_instance::InstanceId),
    UPDATE(edgeless_api::function_instance::UpdateFunctionLinksRequest),
}

pub struct Agent {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
    node_settings: crate::EdgelessNodeSettings,
}

impl Agent {
    pub fn new(
        runner: Box<dyn runner_api::RunnerAPI + Send>,
        node_settings: crate::EdgelessNodeSettings,
    ) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();

        let main_task = Box::pin(async move {
            Self::main_task(receiver, runner).await;
        });

        (Agent { sender, node_settings }, main_task)
    }

    async fn main_task(receiver: futures::channel::mpsc::UnboundedReceiver<AgentRequest>, runner: Box<dyn runner_api::RunnerAPI + Send>) {
        let mut receiver = std::pin::pin!(receiver);
        let mut runner = runner;
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
                AgentRequest::UPDATE(update) => {
                    log::debug!("Agent Update {:?}", update);
                    match runner.update_links(update).await {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled Update Error: {}", err);
                        }
                    }
                }
            }
        }
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::agent::AgentAPI + Send> {
        Box::new(AgentClient {
            function_instance_client: Box::new(FunctionInstanceClient {
                sender: self.sender.clone(),
                node_id: self.node_settings.node_id.clone(),
            }),
        })
    }
}

#[derive(Clone)]
pub struct FunctionInstanceClient {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
    node_id: uuid::Uuid,
}

#[derive(Clone)]
pub struct AgentClient {
    function_instance_client: Box<dyn edgeless_api::function_instance::FunctionInstanceAPI>,
}

#[async_trait::async_trait]
impl edgeless_api::function_instance::FunctionInstanceAPI for FunctionInstanceClient {
    async fn start(
        &mut self,
        request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::function_instance::SpawnFunctionResponse> {
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
            Ok(_) => Ok(edgeless_api::function_instance::SpawnFunctionResponse {
                response_error: None,
                instance_id: Some(f_id),
            }),
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
        match self.sender.send(AgentRequest::UPDATE(update)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Agent channel error when updating the links of a function instance: {}",
                err.to_string()
            )),
        }
    }
}

impl edgeless_api::agent::AgentAPI for AgentClient {
    fn function_instance_api(&mut self) -> Box<dyn edgeless_api::function_instance::FunctionInstanceAPI> {
        self.function_instance_client.clone()
    }
}
