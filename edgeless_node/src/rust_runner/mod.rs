use futures::{SinkExt, StreamExt};

use crate::runner_api;

enum RustRunnerRequest {
    START(edgeless_api::function_instance::FunctionId),
    STOP(edgeless_api::function_instance::FunctionId),
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
                        RustRunnerRequest::START(function_id) => {
                            log::debug!("Runner Start Function {:?}", function_id);
                        }
                        RustRunnerRequest::STOP(function_id) => {
                            log::debug!("Runner Stop Function {:?}", function_id);
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
    async fn start(&mut self, function_id: edgeless_api::function_instance::FunctionId) {
        let _ = self.sender.send(RustRunnerRequest::START(function_id)).await;
    }

    async fn stop(&mut self, function_id: edgeless_api::function_instance::FunctionId) {
        let _ = self.sender.send(RustRunnerRequest::STOP(function_id)).await;
    }
}
