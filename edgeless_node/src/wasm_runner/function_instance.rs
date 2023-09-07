use edgeless_dataplane::core::CallRet;
use futures::{FutureExt, SinkExt};

/// Handle/Wrapper around a single instance of a WASM function.
/// This manages the task executing the function.
/// This is the management interface used to interact with a function instace.
/// The `callback_table` is used to set up the alias-callbacks and is shared with the guest API.
pub struct FunctionInstance {
    task_handle: Option<tokio::task::JoinHandle<()>>,
    callback_table: std::sync::Arc<tokio::sync::Mutex<FunctionInstanceCallbackTable>>,
    stop_handle: Option<futures::channel::oneshot::Sender<()>>,
}

/// State used within the function-instance task. This manages the WASM VM.
struct FunctionInstanceInner {
    instance_id: edgeless_api::function_instance::InstanceId,
    store: wasmtime::Store<super::guest_api::GuestAPI>,
    binding: super::guest_api::wit_binding::Edgefunction,
    data_plane: edgeless_dataplane::handle::DataplaneHandle,
    runner_api: futures::channel::mpsc::UnboundedSender<super::runner::WasmRunnerRequest>,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
}

/// Struct representing the updatable callbacks/aliases of a function instance.
/// Shared between instance api and guest api.
pub struct FunctionInstanceCallbackTable {
    pub alias_map: std::collections::HashMap<String, edgeless_api::function_instance::InstanceId>,
}

impl FunctionInstance {
    pub async fn launch(
        spawn_req: edgeless_api::function_instance::SpawnFunctionRequest,
        data_plane: edgeless_dataplane::handle::DataplaneHandle,
        runner_api: futures::channel::mpsc::UnboundedSender<super::runner::WasmRunnerRequest>,
        state_handle: Box<dyn crate::state_management::StateHandleAPI>,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    ) -> anyhow::Result<Self> {
        let mut telemetry_handle = telemetry_handle;

        let callback_table = std::sync::Arc::new(tokio::sync::Mutex::new(FunctionInstanceCallbackTable {
            alias_map: spawn_req.output_callback_definitions.clone(),
        }));
        let instance_id = match spawn_req.instance_id.clone() {
            Some(id) => id,
            None => {
                return Err(anyhow::anyhow!("No InstanceId!"));
            }
        };

        let cloned_callbacks: std::sync::Arc<tokio::sync::Mutex<FunctionInstanceCallbackTable>> = callback_table.clone();
        let cloned_telemetry = telemetry_handle.fork(std::collections::BTreeMap::new());
        let (stop_sender, stop_receiver) = futures::channel::oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let receiver = stop_receiver;
            if let Ok(mut f) = FunctionInstanceInner::new(
                instance_id.clone(),
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
                log::info!("Function Spawn Error {:?}", instance_id);
            }
        });

        Ok(Self {
            task_handle: Some(task),
            callback_table: callback_table,
            stop_handle: Some(stop_sender),
        })
    }

    pub async fn stop(&mut self) {
        if let Some(poison) = self.stop_handle.take() {
            poison.send(()).unwrap();
        }
        if let Some(handle) = self.task_handle.take() {
            handle.await.unwrap();
        }
    }

    pub async fn update_links(&mut self, update_req: edgeless_api::function_instance::UpdateFunctionLinksRequest) {
        self.callback_table.lock().await.alias_map = update_req.output_callback_definitions;
    }
}

impl FunctionInstanceInner {
    async fn new(
        instance_id: edgeless_api::function_instance::InstanceId,
        binary: &[u8],
        callback_table: std::sync::Arc<tokio::sync::Mutex<FunctionInstanceCallbackTable>>,
        data_plane: edgeless_dataplane::handle::DataplaneHandle,
        runner_api: futures::channel::mpsc::UnboundedSender<super::runner::WasmRunnerRequest>,
        state_handle: Box<dyn crate::state_management::StateHandleAPI>,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    ) -> anyhow::Result<Self> {
        let mut state_handle = state_handle;
        let mut telemetry_handle = telemetry_handle;

        // VM Setup & Initialization
        let start = tokio::time::Instant::now();
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        config.wasm_component_model(true);
        let engine = wasmtime::Engine::new(&config)?;
        let component = wasmtime::component::Component::from_binary(&engine, binary)?;
        let mut linker = wasmtime::component::Linker::new(&engine);
        super::guest_api::wit_binding::Edgefunction::add_to_linker(&mut linker, |state: &mut super::guest_api::GuestAPI| state)?;
        let serialized_state = state_handle.get().await;
        let mut store = wasmtime::Store::new(
            &engine,
            super::guest_api::GuestAPI {
                instance_id: instance_id.clone(),
                data_plane: data_plane.clone(),
                callback_table: callback_table,
                state_handle: state_handle,
                telemetry_handle: telemetry_handle.fork(std::collections::BTreeMap::new()),
            },
        );
        let (binding, _instance) = super::guest_api::wit_binding::Edgefunction::instantiate_async(&mut store, &component, &linker).await?;
        telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInstantiate(start.elapsed()),
            std::collections::BTreeMap::new(),
        );

        // Function Init (Call to the init handler).
        let start = tokio::time::Instant::now();
        binding.call_handle_init(&mut store, "test", serialized_state.as_deref()).await?;
        telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInit(start.elapsed()),
            std::collections::BTreeMap::new(),
        );

        Ok(Self {
            instance_id,
            store,
            binding,
            data_plane,
            runner_api,
            telemetry_handle,
        })
    }

    /// Main task active across the whole lifecycle of the function.
    async fn run(&mut self, stop_event_receiver: futures::channel::oneshot::Receiver<()>) {
        let mut stop_event_receiver = stop_event_receiver;
        // Main Lifecycle
        loop {
            futures::select! {
                _ = stop_event_receiver => {
                    match self.process_stop().await {
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
                            match self.process_cast(source_id, payload).await {
                                Ok(_) => {}
                                Err(_) => {
                                    break;
                                }
                            }
                        }
                        edgeless_dataplane::core::Message::Call(payload) => {
                            match self.process_call(channel_id, source_id, payload).await {
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
        // Function Exit
        match self
            .runner_api
            .send(super::runner::WasmRunnerRequest::FunctionExit(self.instance_id.clone()))
            .await
        {
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

    async fn process_cast(&mut self, src: edgeless_api::function_instance::InstanceId, msg: String) -> anyhow::Result<()> {
        let start = tokio::time::Instant::now();
        self.binding
            .call_handle_cast(
                &mut self.store,
                &super::guest_api::wit_binding::InstanceId {
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

    async fn process_call(&mut self, channel_id: u64, src: edgeless_api::function_instance::InstanceId, msg: String) -> anyhow::Result<()> {
        let start = tokio::time::Instant::now();
        let res = self
            .binding
            .call_handle_call(
                &mut self.store,
                &super::guest_api::wit_binding::InstanceId {
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
                super::guest_api::wit_binding::CallRet::Err => CallRet::Err,
                super::guest_api::wit_binding::CallRet::Noreply => CallRet::NoReply,
                super::guest_api::wit_binding::CallRet::Reply(msg) => CallRet::Reply(msg),
            },
        )
        .await;
        Ok(())
    }

    async fn process_stop(&mut self) -> anyhow::Result<()> {
        let start = tokio::time::Instant::now();
        self.binding.call_handle_stop(&mut self.store).await?;
        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionStop(start.elapsed()),
            std::collections::BTreeMap::new(),
        );
        Ok(())
    }
}
