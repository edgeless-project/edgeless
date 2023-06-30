use futures::{Future, SinkExt, StreamExt};

use crate::runner_api;

enum AgentRequest {
    SPAWN(edgeless_api::function_instance::SpawnFunctionRequest),
    STOP(edgeless_api::function_instance::FunctionId),
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
            Self::processing_loop(receiver, runner).await;
        });

        (Agent { sender, node_settings }, main_task)
    }

    async fn processing_loop(receiver: futures::channel::mpsc::UnboundedReceiver<AgentRequest>, runner: Box<dyn runner_api::RunnerAPI + Send>) {
        let mut receiver = std::pin::pin!(receiver);
        let mut runner = runner;
        log::info!("Starting Edgeless Agent");
        while let Some(req) = receiver.next().await {
            match req {
                AgentRequest::SPAWN(spawn_req) => {
                    log::debug!("Agent Spawn {:?}", spawn_req);
                    runner.start(spawn_req).await;
                }
                AgentRequest::STOP(stop_function_id) => {
                    log::debug!("Agent Stop {:?}", stop_function_id);
                    runner.stop(stop_function_id).await;
                }
                AgentRequest::UPDATE(update) => {
                    log::debug!("Agent Update {:?}", update);
                    runner.update(update).await;
                }
            }
        }
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::agent::AgentAPI + Send> {
        Box::new(AgentClient {
            function_instance_client: Some(Box::new(FunctionInstanceClient {
                sender: self.sender.clone(),
                node_id: self.node_settings.node_id.clone(),
            })),
        })
    }
}

pub struct FunctionInstanceClient {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
    node_id: uuid::Uuid,
}

pub struct AgentClient {
    function_instance_client: Option<Box<dyn edgeless_api::function_instance::FunctionInstanceAPI + Send>>,
}

#[async_trait::async_trait]
impl edgeless_api::function_instance::FunctionInstanceAPI for FunctionInstanceClient {
    async fn start_function_instance(
        &mut self,
        request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::function_instance::FunctionId> {
        let mut request = request;
        if request.function_id.is_none() {
            request.function_id = Some(edgeless_api::function_instance::FunctionId::new(self.node_id));
        }
        let fid = request.function_id.clone().unwrap();
        let _ = self.sender.send(AgentRequest::SPAWN(request)).await;
        Ok(fid)
    }
    async fn stop_function_instance(&mut self, id: edgeless_api::function_instance::FunctionId) -> anyhow::Result<()> {
        let _ = self.sender.send(AgentRequest::STOP(id)).await;
        Ok(())
    }

    async fn update_function_instance_links(&mut self, update: edgeless_api::function_instance::UpdateFunctionLinksRequest) -> anyhow::Result<()> {
        let _ = self.sender.send(AgentRequest::UPDATE(update)).await;
        Ok(())
    }
}

impl edgeless_api::agent::AgentAPI for AgentClient {
    fn function_instance_api(&mut self) -> Box<dyn edgeless_api::function_instance::FunctionInstanceAPI + Send> {
        self.function_instance_client.take().unwrap()
    }
}
