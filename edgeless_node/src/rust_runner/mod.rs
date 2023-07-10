use futures::{SinkExt, StreamExt};

use crate::{
    data_plane::{self},
    runner_api,
};

mod api {
    wasmtime::component::bindgen!({async: true});
}

enum RustRunnerRequest {
    START(edgeless_api::function_instance::SpawnFunctionRequest),
    STOP(edgeless_api::function_instance::FunctionId),
    UPDATE(edgeless_api::function_instance::UpdateFunctionLinksRequest),
}

pub struct Runner {
    sender: futures::channel::mpsc::UnboundedSender<RustRunnerRequest>,
}

impl Runner {
    pub fn new(
        _settings: crate::EdgelessNodeSettings,
        data_plane_provider: data_plane::DataPlaneChainProvider,
    ) -> (Self, futures::future::BoxFuture<'static, ()>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        (
            Runner { sender },
            Box::pin(async move {
                let mut receiver = receiver;
                let mut data_plane_provider = data_plane_provider;
                log::info!("Starting Edgeless Rust Runner");
                while let Some(req) = receiver.next().await {
                    match req {
                        RustRunnerRequest::START(spawn_request) => {
                            let function_id = match spawn_request.function_id.clone() {
                                Some(id) => id,
                                None => {
                                    continue;
                                }
                            };
                            log::debug!("Start Function {:?}", spawn_request);
                            let cloned_req = spawn_request.clone();
                            let handle = data_plane_provider.get_chain_for(function_id.clone()).await;
                            tokio::spawn(async move {
                                if let Ok(mut f) = FunctionInstance::new(cloned_req, handle).await {
                                    f.run().await;
                                } else {
                                    log::info!("Function Spawn Error {:?}", function_id);
                                }
                            });
                        }
                        RustRunnerRequest::STOP(function_id) => {
                            log::debug!("Stop Function {:?}", function_id);
                        }
                        RustRunnerRequest::UPDATE(update) => {
                            log::debug!("Update Function {:?}", update);
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

struct FunctionInstance {
    config: wasmtime::Config,
    engine: wasmtime::Engine,
    component: wasmtime::component::Component,
    linker: wasmtime::component::Linker<FunctionState>,
    store: wasmtime::Store<FunctionState>,
    binding: api::Edgefun,
    instance: wasmtime::component::Instance,
    data_plane: data_plane::DataPlaneChainHandle,
}

struct FunctionState {
    function_id: edgeless_api::function_instance::FunctionId,
    data_plane: data_plane::DataPlaneChainWriteHandle,
    alias_map: std::collections::HashMap<String, edgeless_api::function_instance::FunctionId>,
}

impl FunctionInstance {
    async fn new(
        spawn_req: edgeless_api::function_instance::SpawnFunctionRequest,
        data_plane: data_plane::DataPlaneChainHandle,
    ) -> anyhow::Result<Self> {
        let function_id = match spawn_req.function_id {
            Some(id) => id,
            None => return Err(anyhow::anyhow!("No FunctionId.")),
        };
        let mut data_plane = data_plane;
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        config.wasm_component_model(true);
        let engine = wasmtime::Engine::new(&config)?;
        let component = wasmtime::component::Component::from_binary(&engine, &spawn_req.code.function_class_inlude_code)?;
        let mut linker = wasmtime::component::Linker::new(&engine);
        api::Edgefun::add_to_linker(&mut linker, |state: &mut FunctionState| state)?;
        let mut store = wasmtime::Store::new(
            &engine,
            FunctionState {
                function_id: function_id,
                data_plane: data_plane.new_write_handle().await,
                alias_map: spawn_req.output_callback_definitions,
            },
        );
        let (binding, instance) = api::Edgefun::instantiate_async(&mut store, &component, &linker).await?;
        binding.call_handle_init(&mut store, "test").await?;
        Ok(Self {
            config,
            engine,
            component,
            linker,
            store,
            binding,
            instance,
            data_plane,
        })
    }

    async fn run(&mut self) {
        loop {
            let (src, msg) = self.data_plane.receive_next().await;
            match self.activate(Event::Call(src, msg)).await {
                Ok(_) => {}
                Err(_) => {
                    return;
                }
            }
        }
    }

    async fn activate(&mut self, event: Event) -> anyhow::Result<()> {
        match event {
            Event::Call(src, msg) => {
                self.binding
                    .call_handle_call(
                        &mut self.store,
                        &api::Fid {
                            node: src.node_id.to_string(),
                            function: src.function_id.to_string(),
                        },
                        &msg,
                    )
                    .await?;
                Ok(())
            }
            Event::Stop() => {
                self.binding.call_handle_stop(&mut self.store).await?;
                Ok(())
            }
        }
    }
}

#[async_trait::async_trait]
impl api::EdgefunImports for FunctionState {
    async fn call_alias(&mut self, alias: String, msg: String) -> wasmtime::Result<()> {
        if let Some(target) = self.alias_map.get(&alias) {
            self.data_plane.send(target.clone(), msg).await;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Unknown alias."))
        }
    }

    async fn call(&mut self, target: api::Fid, msg: String) -> wasmtime::Result<()> {
        let parsed_target = parse_wit_function_id(&target)?;
        self.data_plane.send(parsed_target, msg).await;
        Ok(())
    }

    async fn log(&mut self, msg: String) -> wasmtime::Result<()> {
        log::info!("Function Log: {}", msg);
        Ok(())
    }

    async fn slf(&mut self) -> wasmtime::Result<api::Fid> {
        Ok(api::Fid {
            node: self.function_id.node_id.to_string(),
            function: self.function_id.function_id.to_string(),
        })
    }

    async fn delayed_call(&mut self, delay: u64, target: api::Fid, payload: String) -> wasmtime::Result<()> {
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
}

fn parse_wit_function_id(fid: &api::Fid) -> anyhow::Result<edgeless_api::function_instance::FunctionId> {
    Ok(edgeless_api::function_instance::FunctionId {
        node_id: uuid::Uuid::parse_str(&fid.node)?,
        function_id: uuid::Uuid::parse_str(&fid.function)?,
    })
}

enum Event {
    Call(edgeless_api::function_instance::FunctionId, String),
    Stop(),
}
