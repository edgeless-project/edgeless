mod api {
    wasmtime::component::bindgen!({path: "../edgeless_function/wit/edgefunction.wit", async: true});
}

#[cfg(test)]
mod test;

use futures::{FutureExt, SinkExt, StreamExt};

use crate::{runner_api, state_management};
use edgeless_dataplane::core::CallRet;

enum WasmRunnerRequest {
    Start(edgeless_api::function_instance::SpawnFunctionRequest),
    Stop(edgeless_api::function_instance::FunctionId),
    Update(edgeless_api::function_instance::UpdateFunctionLinksRequest),
    FunctionExit(edgeless_api::function_instance::FunctionId),
}

pub struct Runner {
    sender: futures::channel::mpsc::UnboundedSender<WasmRunnerRequest>,
}

impl Runner {
    pub fn new(
        data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
        state_manager: Box<dyn state_management::StateManagerAPI>,
        telemtry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    ) -> (Self, futures::future::BoxFuture<'static, ()>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let cloned_sender = sender.clone();
        (
            Runner { sender },
            Box::pin(async move {
                let mut receiver = receiver;
                let mut data_plane_provider = data_plane_provider;
                let mut functions = std::collections::HashMap::<uuid::Uuid, FunctionInstance>::new();
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
                            let instance = FunctionInstance::launch(
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

    pub fn get_api_client(&mut self) -> Box<dyn runner_api::RunnerAPI + Send> {
        Box::new(RunnerClient { sender: self.sender.clone() })
    }
}

struct RunnerClient {
    sender: futures::channel::mpsc::UnboundedSender<WasmRunnerRequest>,
}

#[async_trait::async_trait]
impl runner_api::RunnerAPI for RunnerClient {
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

struct FunctionInstanceTaskState {
    function_id: edgeless_api::function_instance::FunctionId,
    // config: wasmtime::Config,
    // engine: wasmtime::Engine,
    // component: wasmtime::component::Component,
    // linker: wasmtime::component::Linker<FunctionState>,
    store: wasmtime::Store<FunctionState>,
    binding: api::Edgefunction,
    // instance: wasmtime::component::Instance,
    data_plane: edgeless_dataplane::handle::DataplaneHandle,
    runner_api: futures::channel::mpsc::UnboundedSender<WasmRunnerRequest>,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
}

struct FunctionInstanceCallbackTable {
    alias_map: std::collections::HashMap<String, edgeless_api::function_instance::FunctionId>,
}

struct FunctionInstance {
    task_handle: Option<tokio::task::JoinHandle<()>>,
    callback_table: std::sync::Arc<tokio::sync::Mutex<FunctionInstanceCallbackTable>>,
    stop_handle: Option<futures::channel::oneshot::Sender<()>>,
}

impl FunctionInstance {
    async fn launch(
        spawn_req: edgeless_api::function_instance::SpawnFunctionRequest,
        data_plane: edgeless_dataplane::handle::DataplaneHandle,
        runner_api: futures::channel::mpsc::UnboundedSender<WasmRunnerRequest>,
        state_handle: Box<dyn state_management::StateHandleAPI>,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    ) -> anyhow::Result<Self> {
        let mut telemetry_handle = telemetry_handle;
        let callback_table = std::sync::Arc::new(tokio::sync::Mutex::new(FunctionInstanceCallbackTable {
            alias_map: spawn_req.output_callback_definitions.clone(),
        }));
        let function_id = match spawn_req.function_id.clone() {
            Some(id) => id,
            None => {
                return Err(anyhow::anyhow!("No FunctionId!"));
            }
        };
        let cloned_callbacks: std::sync::Arc<tokio::sync::Mutex<FunctionInstanceCallbackTable>> = callback_table.clone();
        let cloned_telemetry = telemetry_handle.fork(std::collections::BTreeMap::new());
        let (sender, receiver) = futures::channel::oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let receiver = receiver;
            if let Ok(mut f) = FunctionInstanceTaskState::new(
                function_id.clone(),
                &spawn_req.code.function_class_inlude_code,
                cloned_callbacks,
                data_plane,
                runner_api,
                state_handle,
                cloned_telemetry,
            )
            .await
            {
                f.run(receiver).await;
            } else {
                log::info!("Function Spawn Error {:?}", function_id);
            }
        });
        Ok(Self {
            task_handle: Some(task),
            callback_table: callback_table,
            stop_handle: Some(sender),
        })
    }

    async fn stop(&mut self) {
        if let Some(poison) = self.stop_handle.take() {
            poison.send(()).unwrap();
        }
        if let Some(handle) = self.task_handle.take() {
            handle.await.unwrap();
        }
    }

    async fn update(&mut self, update_req: edgeless_api::function_instance::UpdateFunctionLinksRequest) {
        self.callback_table.lock().await.alias_map = update_req.output_callback_definitions;
    }
}

struct FunctionState {
    function_id: edgeless_api::function_instance::FunctionId,
    data_plane: edgeless_dataplane::handle::DataplaneHandle,
    callback_table: std::sync::Arc<tokio::sync::Mutex<FunctionInstanceCallbackTable>>,
    state_handle: Box<dyn state_management::StateHandleAPI>,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
}

impl FunctionInstanceTaskState {
    async fn new(
        function_id: edgeless_api::function_instance::FunctionId,
        binary: &[u8],
        callback_table: std::sync::Arc<tokio::sync::Mutex<FunctionInstanceCallbackTable>>,
        data_plane: edgeless_dataplane::handle::DataplaneHandle,
        runner_api: futures::channel::mpsc::UnboundedSender<WasmRunnerRequest>,
        state_handle: Box<dyn state_management::StateHandleAPI>,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    ) -> anyhow::Result<Self> {
        let start = tokio::time::Instant::now();
        let mut config = wasmtime::Config::new();
        let mut state_handle = state_handle;
        let mut telemetry_handle = telemetry_handle;
        config.async_support(true);
        config.wasm_component_model(true);
        let engine = wasmtime::Engine::new(&config)?;
        let component = wasmtime::component::Component::from_binary(&engine, binary)?;
        let mut linker = wasmtime::component::Linker::new(&engine);
        api::Edgefunction::add_to_linker(&mut linker, |state: &mut FunctionState| state)?;
        let serialized_state = state_handle.get().await;
        let mut store = wasmtime::Store::new(
            &engine,
            FunctionState {
                function_id: function_id.clone(),
                data_plane: data_plane.clone(),
                callback_table: callback_table,
                state_handle: state_handle,
                telemetry_handle: telemetry_handle.fork(std::collections::BTreeMap::new()),
            },
        );
        let (binding, _instance) = api::Edgefunction::instantiate_async(&mut store, &component, &linker).await?;
        telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInstantiate(start.elapsed()),
            std::collections::BTreeMap::new(),
        );
        let start = tokio::time::Instant::now();
        binding.call_handle_init(&mut store, "test", serialized_state.as_deref()).await?;
        telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInit(start.elapsed()),
            std::collections::BTreeMap::new(),
        );
        Ok(Self {
            function_id,
            // config,
            // engine,
            // component,
            // linker,
            store,
            binding,
            // instance,
            data_plane,
            runner_api,
            telemetry_handle,
        })
    }

    async fn run(&mut self, poison_pill_receiver: futures::channel::oneshot::Receiver<()>) {
        let mut poison_pill_receiver = poison_pill_receiver;
        loop {
            futures::select! {
                _ = poison_pill_receiver => {
                    match self.activate(Event::Stop).await {
                        Ok(_) => {}
                        Err(_) => {
                            break;
                        }
                    }
                    break;
                },
                edgeless_dataplane::core::DataplaneEvent{source_id, channel_id, message} =  Box::pin(self.data_plane.receive_next()).fuse() => {
                    match message {
                        edgeless_dataplane::core::Message::Cast(payload) => {
                            match self.activate(Event::Cast(source_id, payload)).await {
                                Ok(_) => {}
                                Err(_) => {
                                    break;
                                }
                            }
                        }
                        edgeless_dataplane::core::Message::Call(payload) => {
                            match self.activate(Event::Call(channel_id, source_id, payload)).await {
                                Ok(_) => {}
                                Err(_) => {
                                    break;
                                }
                            }
                        },
                        _ => {
                            log::debug!("Unprocessed Message");
                        }
                    }
                }
            }
        }
        match self.runner_api.send(WasmRunnerRequest::FunctionExit(self.function_id.clone())).await {
            Ok(_) => {}
            Err(_) => {
                log::error!("FunctionInstance outlived runner.")
            }
        };
        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionExit,
            std::collections::BTreeMap::new(),
        );
    }

    async fn activate(&mut self, event: Event) -> anyhow::Result<()> {
        match event {
            Event::Cast(src, msg) => {
                let start = tokio::time::Instant::now();
                self.binding
                    .call_handle_cast(
                        &mut self.store,
                        &api::Fid {
                            node: src.node_id.to_string(),
                            function: src.function_id.to_string(),
                        },
                        &msg,
                    )
                    .await?;
                self.telemetry_handle.observe(
                    edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInvocationCompleted(start.elapsed()),
                    std::collections::BTreeMap::from([("EVENT_TYPE".to_string(), "CAST".to_string())]),
                );
                Ok(())
            }

            Event::Call(channel_id, src, msg) => {
                let start = tokio::time::Instant::now();
                let res = self
                    .binding
                    .call_handle_call(
                        &mut self.store,
                        &api::Fid {
                            node: src.node_id.to_string(),
                            function: src.function_id.to_string(),
                        },
                        &msg,
                    )
                    .await?;
                self.telemetry_handle.observe(
                    edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInvocationCompleted(start.elapsed()),
                    std::collections::BTreeMap::from([("EVENT_TYPE".to_string(), "CALL".to_string())]),
                );
                let mut wh = self.data_plane.clone();
                wh.reply(
                    src,
                    channel_id,
                    match res {
                        api::CallRet::Err => CallRet::Err,
                        api::CallRet::Noreply => CallRet::NoReply,
                        api::CallRet::Reply(msg) => CallRet::Reply(msg),
                    },
                )
                .await;
                Ok(())
            }

            Event::Stop => {
                let start = tokio::time::Instant::now();
                self.binding.call_handle_stop(&mut self.store).await?;
                self.telemetry_handle.observe(
                    edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionStop(start.elapsed()),
                    std::collections::BTreeMap::new(),
                );
                Ok(())
            }
        }
    }
}

#[async_trait::async_trait]
impl api::EdgefunctionImports for FunctionState {
    async fn cast_alias(&mut self, alias: String, msg: String) -> wasmtime::Result<()> {
        if let Some(target) = self.callback_table.lock().await.alias_map.get(&alias) {
            self.data_plane.send(target.clone(), msg).await;
            Ok(())
        } else {
            log::warn!("Unknown alias.");
            Ok(())
        }
    }

    async fn cast(&mut self, target: api::Fid, msg: String) -> wasmtime::Result<()> {
        let parsed_target = parse_wit_function_id(&target)?;
        self.data_plane.send(parsed_target, msg).await;
        Ok(())
    }

    async fn call(&mut self, target: api::Fid, msg: String) -> wasmtime::Result<api::CallRet> {
        let parsed_target = parse_wit_function_id(&target)?;
        let res = self.data_plane.call(parsed_target, msg).await;
        Ok(match res {
            CallRet::Reply(msg) => api::CallRet::Reply(msg),
            CallRet::NoReply => api::CallRet::Noreply,
            CallRet::Err => api::CallRet::Err,
        })
    }

    async fn call_alias(&mut self, alias: String, msg: String) -> wasmtime::Result<api::CallRet> {
        if let Some(target) = self.callback_table.lock().await.alias_map.get(&alias) {
            let res = self.data_plane.call(target.clone(), msg).await;
            Ok(match res {
                CallRet::Reply(msg) => api::CallRet::Reply(msg),
                CallRet::NoReply => api::CallRet::Noreply,
                CallRet::Err => api::CallRet::Err,
            })
        } else {
            log::warn!("Unknown alias.");
            Ok(api::CallRet::Err)
        }
    }

    async fn telemetry_log(&mut self, lvl: String, target: String, msg: String) -> wasmtime::Result<()> {
        let parsed_level = edgeless_telemetry::telemetry_events::api_to_telemetry(lvl);
        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionLogEntry(parsed_level, target, msg),
            std::collections::BTreeMap::new(),
        );
        Ok(())
    }

    async fn slf(&mut self) -> wasmtime::Result<api::Fid> {
        Ok(api::Fid {
            node: self.function_id.node_id.to_string(),
            function: self.function_id.function_id.to_string(),
        })
    }

    async fn delayed_cast(&mut self, delay: u64, target: api::Fid, payload: String) -> wasmtime::Result<()> {
        let mut cloned_plane = self.data_plane.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            if let Ok(parsed_target) = parse_wit_function_id(&target) {
                cloned_plane.send(parsed_target, payload).await;
            } else {
                log::error!("Unhandled failure in delayed message.")
            }
        });
        Ok(())
    }

    async fn sync(&mut self, serialized_state: String) -> wasmtime::Result<()> {
        self.state_handle.set(serialized_state.clone()).await;
        log::info!("Function State Sync: {}", serialized_state);
        Ok(())
    }
}

fn parse_wit_function_id(fid: &api::Fid) -> anyhow::Result<edgeless_api::function_instance::FunctionId> {
    Ok(edgeless_api::function_instance::FunctionId {
        node_id: uuid::Uuid::parse_str(&fid.node)?,
        function_id: uuid::Uuid::parse_str(&fid.function)?,
    })
}

enum Event {
    Cast(edgeless_api::function_instance::FunctionId, String),
    Call(u64, edgeless_api::function_instance::FunctionId, String),
    Stop,
}
