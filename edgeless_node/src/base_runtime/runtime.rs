// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT
/// Generic function runtime hosting a set of runners of one type (e.g. WASM/Docker)
/// Split into the active component `RuntimeTask` and the cloneable `RuntimeClient` allowing to interact with the runtime.
use futures::{SinkExt, StreamExt};

pub trait GuestAPIHostRegister {
    fn needs_to_register(&mut self) -> bool;

    fn register_guest_api_host(
        &mut self,
        instance_id: &edgeless_api::function_instance::InstanceId,
        guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
    );

    fn deregister_guest_api_host(
        &mut self,
        instance_id: &edgeless_api::function_instance::InstanceId,
    );

    fn guest_api_host(
        &mut self,
        instance_id: &edgeless_api::function_instance::InstanceId,
    ) -> Option<&mut crate::base_runtime::guest_api::GuestAPIHost>;

    fn configuration(&mut self) -> std::collections::HashMap<String, String>;
}

#[derive(Clone)]
pub struct RuntimeClient {
    sender: futures::channel::mpsc::UnboundedSender<RuntimeRequest>,
}

pub struct RuntimeTask<FunctionInstanceType: super::FunctionInstance> {
    receiver: futures::channel::mpsc::UnboundedReceiver<RuntimeRequest>,
    data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
    state_manager: Box<dyn crate::state_management::StateManagerAPI>,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    guest_api_host_register:
        std::sync::Arc<tokio::sync::Mutex<Box<dyn GuestAPIHostRegister + Send>>>,
    slf_channel: futures::channel::mpsc::UnboundedSender<RuntimeRequest>,
    functions: std::collections::HashMap<
        uuid::Uuid,
        super::function_instance_runner::FunctionInstanceRunner<FunctionInstanceType>,
    >,
}

pub enum RuntimeRequest {
    Start(
        edgeless_api::function_instance::InstanceId,
        edgeless_api::function_instance::SpawnFunctionRequest,
    ),
    Stop(edgeless_api::function_instance::InstanceId),
    Patch(edgeless_api::common::PatchRequest),
    FunctionExit(
        edgeless_api::function_instance::InstanceId,
        Result<(), super::FunctionInstanceError>,
    ),
}

/// Entrypoint for all runtimes based on the base_runtime.
pub fn create<FunctionInstanceType: super::FunctionInstance>(
    data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
    state_manager: Box<dyn crate::state_management::StateManagerAPI>,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    guest_api_host_register: std::sync::Arc<
        tokio::sync::Mutex<Box<dyn GuestAPIHostRegister + Send>>,
    >,
) -> (RuntimeClient, RuntimeTask<FunctionInstanceType>) {
    let (sender, receiver) = futures::channel::mpsc::unbounded();
    let task: RuntimeTask<FunctionInstanceType> = RuntimeTask::new(
        receiver,
        data_plane_provider,
        state_manager,
        telemetry_handle,
        guest_api_host_register,
        sender.clone(),
    );

    let client = RuntimeClient::new(sender);

    (client, task)
}

impl<FunctionInstanceType: super::FunctionInstance> RuntimeTask<FunctionInstanceType> {
    fn new(
        receiver: futures::channel::mpsc::UnboundedReceiver<RuntimeRequest>,
        data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
        state_manager: Box<dyn crate::state_management::StateManagerAPI>,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        guest_api_host_register: std::sync::Arc<
            tokio::sync::Mutex<Box<dyn GuestAPIHostRegister + Send>>,
        >,
        slf_channel: futures::channel::mpsc::UnboundedSender<RuntimeRequest>,
    ) -> Self {
        Self {
            receiver,
            data_plane_provider,
            state_manager,
            telemetry_handle,
            guest_api_host_register,
            slf_channel,
            functions: std::collections::HashMap::new(),
        }
    }

    pub async fn run(&mut self) {
        log::info!("Starting Edgeless Runner");
        while let Some(req) = self.receiver.next().await {
            match req {
                RuntimeRequest::Start(instance_id, spawn_request) => {
                    self.start_function(instance_id, spawn_request).await;
                }
                RuntimeRequest::Stop(instance_id) => {
                    self.stop_function(instance_id).await;
                }
                RuntimeRequest::Patch(update_request) => {
                    self.patch_function_links(update_request).await;
                }
                RuntimeRequest::FunctionExit(id, status) => {
                    self.function_exit(id, status).await;
                }
            }
        }
    }

    async fn start_function(
        &mut self,
        instance_id: edgeless_api::function_instance::InstanceId,
        spawn_request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) {
        log::info!("Start Function {:?}", instance_id);
        let cloned_req = spawn_request.clone();
        let data_plane = self.data_plane_provider.get_handle_for(instance_id).await;
        let instance = super::function_instance_runner::FunctionInstanceRunner::new(
            instance_id,
            cloned_req,
            data_plane,
            self.slf_channel.clone(),
            self.state_manager
                .get_handle(
                    spawn_request.state_specification.state_policy,
                    spawn_request.state_specification.state_id,
                )
                .await,
            self.telemetry_handle
                .fork(std::collections::BTreeMap::from([(
                    "FUNCTION_ID".to_string(),
                    instance_id.function_id.to_string(),
                )])),
            self.guest_api_host_register.clone(),
        )
        .await;
        self.functions.insert(instance_id.function_id, instance);
    }

    async fn stop_function(&mut self, instance_id: edgeless_api::function_instance::InstanceId) {
        log::info!("Stop Function {:?}", instance_id);
        if let Some(instance) = self.functions.get_mut(&instance_id.function_id) {
            instance.stop().await;
        }
    }

    async fn patch_function_links(&mut self, update_request: edgeless_api::common::PatchRequest) {
        log::info!("Patch Function {:?}", update_request.function_id);
        if let Some(instance) = self.functions.get_mut(&update_request.function_id) {
            instance.patch(update_request).await;
        }
    }

    async fn function_exit(
        &mut self,
        instance_id: edgeless_api::function_instance::InstanceId,
        status: Result<(), super::FunctionInstanceError>,
    ) {
        log::info!("Function Exit Event: {:?} {:?}", instance_id, status);
        self.functions.remove(&instance_id.function_id);
    }
}

impl RuntimeClient {
    pub fn new(
        runtime_request_sender: futures::channel::mpsc::UnboundedSender<RuntimeRequest>,
    ) -> Self {
        RuntimeClient {
            sender: runtime_request_sender,
        }
    }
}

#[async_trait::async_trait]
impl super::RuntimeAPI for RuntimeClient {
    async fn start(
        &mut self,
        instance_id: edgeless_api::function_instance::InstanceId,
        request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<()> {
        match self
            .sender
            .send(RuntimeRequest::Start(instance_id, request))
            .await
        {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Runner Channel Error")),
        }
    }

    async fn stop(
        &mut self,
        instance_id: edgeless_api::function_instance::InstanceId,
    ) -> anyhow::Result<()> {
        match self.sender.send(RuntimeRequest::Stop(instance_id)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Runner Channel Error")),
        }
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        match self.sender.send(RuntimeRequest::Patch(update)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Runner Channel Error")),
        }
    }
}
