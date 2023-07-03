use futures::{SinkExt, StreamExt};

use crate::runner_api;

enum RustRunnerRequest {
    START(edgeless_api::function_instance::SpawnFunctionRequest),
    STOP(edgeless_api::function_instance::FunctionId),
    UPDATE(edgeless_api::function_instance::UpdateFunctionLinksRequest),
}

pub struct Runner {
    sender: futures::channel::mpsc::UnboundedSender<RustRunnerRequest>,
}

impl Runner {
    pub fn new(_settings: crate::EdgelessNodeSettings) -> (Self, futures::future::BoxFuture<'static, ()>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        (
            Runner { sender },
            Box::pin(async move {
                let mut receiver = receiver;
                log::info!("Starting Edgeless Rust Runner");
                while let Some(req) = receiver.next().await {
                    match req {
                        RustRunnerRequest::START(spawn_request) => {
                            log::debug!("Runner Start {:?}", spawn_request);
                        }
                        RustRunnerRequest::STOP(function_id) => {
                            log::debug!("Runner Stop {:?}", function_id);
                        }
                        RustRunnerRequest::UPDATE(update) => {
                            log::debug!("Runner Update {:?}", update);
                        }
                    }
                }
            }),
        )
    }

    pub fn get_api_client(&mut self) -> Box<dyn runner_api::RunnerAPI + Send> {
        Box::new(RunnerClient { sender: self.sender.clone() })
    }
}

struct RunnerClient {
    sender: futures::channel::mpsc::UnboundedSender<RustRunnerRequest>,
}

#[async_trait::async_trait]
impl runner_api::RunnerAPI for RunnerClient {
    async fn start(&mut self, request: edgeless_api::function_instance::SpawnFunctionRequest) -> anyhow::Result<()> {
        match self.sender.send(RustRunnerRequest::START(request)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Runner Channel Error")),
        }
    }

    async fn stop(&mut self, function_id: edgeless_api::function_instance::FunctionId) -> anyhow::Result<()> {
        match self.sender.send(RustRunnerRequest::STOP(function_id)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Runner Channel Error")),
        }
    }

    async fn update(&mut self, update: edgeless_api::function_instance::UpdateFunctionLinksRequest) -> anyhow::Result<()> {
        match self.sender.send(RustRunnerRequest::UPDATE(update)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Runner Channel Error")),
        }
    }
}

}
