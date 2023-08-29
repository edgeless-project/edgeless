use futures::{SinkExt, StreamExt};

pub struct Runner {}

#[derive(Clone)]
pub struct RunnerClient {
    sender: futures::channel::mpsc::UnboundedSender<WasmRunnerRequest>,
}

pub enum WasmRunnerRequest {
    Start(edgeless_api::function_instance::SpawnFunctionRequest),
    Stop(edgeless_api::function_instance::FunctionId),
    Update(edgeless_api::function_instance::UpdateFunctionLinksRequest),
    FunctionExit(edgeless_api::function_instance::FunctionId),
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
                            let function_id = match spawn_request.function_id.clone() {
                                Some(id) => id,
                                None => {
                                    continue;
                                }
                            };
                            log::info!("Start Function {:?}", spawn_request.function_id);
                            let cloned_req = spawn_request.clone();
                            let data_plane = data_plane_provider.get_handle_for(function_id.clone()).await;
                            let instance = super::function_instance::FunctionInstance::launch(
                                cloned_req,
                                data_plane,
                                cloned_sender.clone(),
                                state_manager
                                    .get_handle(spawn_request.state_specification.state_policy, spawn_request.state_specification.state_id)
                                    .await,
                                telemetry_handle.fork(std::collections::BTreeMap::from([(
                                    "FUNCTION_ID".to_string(),
                                    function_id.function_id.to_string(),
                                )])),
                            )
                            .await;
                            functions.insert(function_id.function_id.clone(), instance.unwrap());
                        }
                        WasmRunnerRequest::Stop(function_id) => {
                            log::info!("Stop Function {:?}", function_id);
                            if let Some(instance) = functions.get_mut(&function_id.function_id) {
                                instance.stop().await;
                            }
                            // This will also create a FUNCTION_EXIT event.
                            functions.remove(&function_id.function_id);
                        }
                        WasmRunnerRequest::Update(update) => {
                            log::info!("Update Function {:?}", update.function_id);
                            if let Some(instance) = functions.get_mut(&update.function_id.as_ref().unwrap().function_id) {
                                instance.update(update).await;
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

    async fn stop(&mut self, function_id: edgeless_api::function_instance::FunctionId) -> anyhow::Result<()> {
        match self.sender.send(WasmRunnerRequest::Stop(function_id)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Runner Channel Error")),
        }
    }

    async fn update(&mut self, update: edgeless_api::function_instance::UpdateFunctionLinksRequest) -> anyhow::Result<()> {
        match self.sender.send(WasmRunnerRequest::Update(update)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Runner Channel Error")),
        }
    }
}