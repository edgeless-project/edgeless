use futures::{SinkExt, StreamExt};

pub struct Runner {}

#[derive(Clone)]
pub struct RunnerClient {
    sender: futures::channel::mpsc::UnboundedSender<WasmRunnerRequest>,
}

pub enum WasmRunnerRequest {
    Start(edgeless_api::function_instance::SpawnFunctionRequest),
    Stop(edgeless_api::function_instance::InstanceId),
    Patch(edgeless_api::function_instance::PatchRequest),
    FunctionExit(edgeless_api::function_instance::InstanceId),
}

impl Runner {
    pub fn new(
        data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
        state_manager: Box<dyn crate::state_management::StateManagerAPI>,
        telemtry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    ) -> (RunnerClient, futures::future::BoxFuture<'static, ()>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let cloned_sender = sender.clone();
        (
            RunnerClient { sender },
            Box::pin(async move {
                let mut receiver = receiver;
                let mut data_plane_provider = data_plane_provider;
                let mut functions = std::collections::HashMap::<uuid::Uuid, super::function_instance::FunctionInstance>::new();
                let mut state_manager = state_manager;
                let mut telemetry_handle = telemtry_handle;
                log::info!("Starting Edgeless WASM Runner");
                while let Some(req) = receiver.next().await {
                    match req {
                        WasmRunnerRequest::Start(spawn_request) => {
                            let instance_id = match spawn_request.instance_id.clone() {
                                Some(id) => id,
                                None => {
                                    continue;
                                }
                            };
                            log::info!("Start Function {:?}", spawn_request.instance_id);
                            let cloned_req = spawn_request.clone();
                            let data_plane = data_plane_provider.get_handle_for(instance_id.clone()).await;
                            let instance = super::function_instance::FunctionInstance::launch(
                                cloned_req,
                                data_plane,
                                cloned_sender.clone(),
                                state_manager
                                    .get_handle(spawn_request.state_specification.state_policy, spawn_request.state_specification.state_id)
                                    .await,
                                telemetry_handle.fork(std::collections::BTreeMap::from([(
                                    "FUNCTION_ID".to_string(),
                                    instance_id.function_id.to_string(),
                                )])),
                            )
                            .await;
                            functions.insert(instance_id.function_id.clone(), instance.unwrap());
                        }
                        WasmRunnerRequest::Stop(instance_id) => {
                            log::info!("Stop Function {:?}", instance_id);
                            if let Some(instance) = functions.get_mut(&instance_id.function_id) {
                                instance.stop().await;
                            }
                            // This will also create a FUNCTION_EXIT event.
                            functions.remove(&instance_id.function_id);
                        }
                        WasmRunnerRequest::Patch(update) => {
                            log::info!("Patch Function {:?}", update);
                            if let Some(instance) = functions.get_mut(&update.function_id) {
                                instance.patch(update).await;
                            }
                        }
                        WasmRunnerRequest::FunctionExit(id) => {
                            log::info!("Function Exit Event: {:?}", id);
                        }
                    }
                }
            }),
        )
    }
}

#[async_trait::async_trait]
impl crate::runner_api::RunnerAPI for RunnerClient {
    async fn start(&mut self, request: edgeless_api::function_instance::SpawnFunctionRequest) -> anyhow::Result<()> {
        match self.sender.send(WasmRunnerRequest::Start(request)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Runner Channel Error")),
        }
    }

    async fn stop(&mut self, instance_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        match self.sender.send(WasmRunnerRequest::Stop(instance_id)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Runner Channel Error")),
        }
    }

    async fn patch(&mut self, update: edgeless_api::function_instance::PatchRequest) -> anyhow::Result<()> {
        match self.sender.send(WasmRunnerRequest::Patch(update)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Runner Channel Error")),
        }
    }
}
