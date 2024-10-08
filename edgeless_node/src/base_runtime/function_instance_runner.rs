// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use futures::{FutureExt, SinkExt};
use std::marker::PhantomData;

use super::{FunctionInstance, FunctionInstanceError};

/// This is the main interface for executing/managing a function instance.
/// Owning client for a single function instance task.
/// It is generic over the runtime technology (e.g. WASM).
/// FunctionInstanceRunner (with it's FunctionInstanceTask) do most of the heavy lifting/lifetime management,
/// while the technology specific implementations implement `FunctionInstance` interact and bind a virtualization technology.
pub struct FunctionInstanceRunner<FunctionInstanceType: FunctionInstance> {
    task_handle: Option<tokio::task::JoinHandle<()>>,
    data_plane: edgeless_dataplane::handle::DataplaneHandle,
    poison_pill_sender: tokio::sync::broadcast::Sender<()>,
    _instance: PhantomData<FunctionInstanceType>,
}

/// This is a runnable object (with all required state) actually executing a function.
/// It is managed/owned by a FunctionInstanceRunner, which also runs it using a tokio task.
struct FunctionInstanceTask<FunctionInstanceType: FunctionInstance> {
    poison_pill_receiver: tokio::sync::broadcast::Receiver<()>,
    function_instance: Option<Box<FunctionInstanceType>>,
    guest_api_host: Option<super::guest_api::GuestAPIHost>,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    guest_api_host_register: std::sync::Arc<tokio::sync::Mutex<Box<dyn super::runtime::GuestAPIHostRegister + Send>>>,
    code: Vec<u8>,
    data_plane: edgeless_dataplane::handle::DataplaneHandle,
    serialized_state: Option<String>,
    init_payload: Option<String>,
    runtime_api: futures::channel::mpsc::UnboundedSender<super::runtime::RuntimeRequest>,
    instance_id: edgeless_api::function_instance::InstanceId,
}

impl<FunctionInstanceType: FunctionInstance> FunctionInstanceRunner<FunctionInstanceType> {
    pub async fn new(
        spawn_req: edgeless_api::function_instance::SpawnFunctionRequest,
        data_plane: edgeless_dataplane::handle::DataplaneHandle,
        runtime_api: futures::channel::mpsc::UnboundedSender<super::runtime::RuntimeRequest>,
        state_handle: Box<dyn crate::state_management::StateHandleAPI>,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        guest_api_host_register: std::sync::Arc<tokio::sync::Mutex<Box<dyn super::runtime::GuestAPIHostRegister + Send>>>,
    ) -> Self {
        let instance_id = spawn_req.instance_id;
        let mut telemetry_handle = telemetry_handle;
        let mut state_handle = state_handle;

        // alias_mapping.update(spawn_req.output_mapping).await;
        let (poison_pill_sender, poison_pill_receiver) = tokio::sync::broadcast::channel::<()>(1);
        let serialized_state = state_handle.get().await;

        let guest_api_host = crate::base_runtime::guest_api::GuestAPIHost {
            instance_id: instance_id.clone(),
            data_plane: data_plane.clone(),
            state_handle,
            telemetry_handle: telemetry_handle.fork(std::collections::BTreeMap::new()),
            poison_pill_receiver: poison_pill_sender.subscribe(),
        };

        let task = Box::new(
            FunctionInstanceTask::<FunctionInstanceType>::new(
                poison_pill_receiver,
                telemetry_handle,
                guest_api_host_register,
                guest_api_host,
                spawn_req.code.function_class_code.clone(),
                data_plane.clone(),
                serialized_state,
                spawn_req.annotations.get("init-payload").map(|x| x.clone()),
                runtime_api,
                instance_id.clone(),
            )
            .await,
        );

        let task_handle = tokio::task::spawn(async move {
            let mut task = task;
            task.run().await;
        });

        Self {
            task_handle: Some(task_handle),
            poison_pill_sender,
            data_plane: data_plane.clone(),
            _instance: PhantomData {},
        }
    }

    pub async fn stop(&mut self) {
        self.poison_pill_sender.send(()).unwrap();

        if let Some(handle) = self.task_handle.take() {
            handle.await.unwrap();
        }
    }

    pub async fn patch(&mut self, update_request: edgeless_api::common::PatchRequest) {
        self.data_plane
            .update_mapping(update_request.input_mapping, update_request.output_mapping)
            .await;
    }
}

impl<FunctionInstanceType: FunctionInstance> FunctionInstanceTask<FunctionInstanceType> {
    pub async fn new(
        poison_pill_receiver: tokio::sync::broadcast::Receiver<()>,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        guest_api_host_register: std::sync::Arc<tokio::sync::Mutex<Box<dyn super::runtime::GuestAPIHostRegister + Send>>>,
        guest_api_host: super::guest_api::GuestAPIHost,
        code: Vec<u8>,
        data_plane: edgeless_dataplane::handle::DataplaneHandle,
        serialized_state: Option<String>,
        init_param: Option<String>,
        runtime_api: futures::channel::mpsc::UnboundedSender<super::runtime::RuntimeRequest>,
        instance_id: edgeless_api::function_instance::InstanceId,
    ) -> Self {
        Self {
            poison_pill_receiver,
            function_instance: None,
            guest_api_host: Some(guest_api_host),
            telemetry_handle,
            guest_api_host_register,
            code,
            data_plane,
            serialized_state,
            init_payload: init_param,
            runtime_api,
            instance_id,
        }
    }

    /// Function lifecycle; Runs until the poison pill is received or there is an error.
    /// Always calls the exit handler (with the exit status)
    pub async fn run(&mut self) {
        let mut res = self.instantiate().await;
        assert!(self.guest_api_host.is_none());
        if res.is_ok() {
            res = self.init().await;
        }
        if res.is_ok() {
            res = self.processing_loop().await;
        }
        self.guest_api_host_register.lock().await.deregister_guest_api_host(&self.instance_id);
        self.exit(res).await;
    }

    async fn instantiate(&mut self) -> Result<(), super::FunctionInstanceError> {
        let start = tokio::time::Instant::now();

        let runtime_configuration;
        {
            // Register this function instance, if needed by the runtime.
            let mut register = self.guest_api_host_register.lock().await;
            if register.needs_to_register() {
                register.register_guest_api_host(&self.instance_id, self.guest_api_host.take().unwrap());
            }
            runtime_configuration = register.configuration();
        }

        self.function_instance =
            Some(FunctionInstanceType::instantiate(&self.instance_id, runtime_configuration, &mut self.guest_api_host.take(), &self.code).await?);

        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInstantiate(start.elapsed()),
            std::collections::BTreeMap::new(),
        );

        Ok(())
    }

    async fn init(&mut self) -> Result<(), super::FunctionInstanceError> {
        let start = tokio::time::Instant::now();

        self.function_instance
            .as_mut()
            .ok_or(super::FunctionInstanceError::InternalError)?
            .init(
                self.init_payload.as_ref().map(|x| x.as_str()),
                self.serialized_state.as_ref().map(|x| x.as_str()),
            )
            .await?;

        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInit(start.elapsed()),
            std::collections::BTreeMap::new(),
        );

        Ok(())
    }

    async fn processing_loop(&mut self) -> Result<(), super::FunctionInstanceError> {
        // let mut poison_pill_recv = Box::pin(self.poison_pill_receiver.recv()).fuse();
        loop {
            futures::select! {
                // Given each function instance is an independent task, the runtime needs to send a poison pill to cleanly stop it (processed here)
                _ = Box::pin(self.poison_pill_receiver.recv()).fuse() => {
                    return self.stop().await;
                },
                // Receive a normal event from the dataplane and invoke the function instance
                edgeless_dataplane::core::DataplaneEvent{source_id, channel_id, message, target_port} =  Box::pin(self.data_plane.receive_next()).fuse() => {
                    self.process_message(
                        source_id,
                        channel_id,
                        message,
                        target_port
                    ).await?;
                }
            }
        }
    }

    async fn process_message(
        &mut self,
        source_id: edgeless_api::function_instance::InstanceId,
        channel_id: u64,
        message: edgeless_dataplane::core::Message,
        target_port: edgeless_api::function_instance::PortId,
    ) -> Result<(), super::FunctionInstanceError> {
        match message {
            edgeless_dataplane::core::Message::Cast(payload) => self.process_cast_message(source_id.clone(), target_port, payload).await,
            edgeless_dataplane::core::Message::Call(payload) => self.process_call_message(source_id, target_port, payload, channel_id).await,
            _ => {
                log::debug!("Unprocessed Message");
                Ok(())
            }
        }
    }

    async fn process_cast_message(
        &mut self,
        source_id: edgeless_api::function_instance::InstanceId,
        target_port: edgeless_api::function_instance::PortId,
        payload: String,
    ) -> Result<(), super::FunctionInstanceError> {
        let start = tokio::time::Instant::now();

        self.function_instance
            .as_mut()
            .ok_or(super::FunctionInstanceError::InternalError)?
            .cast(&source_id, target_port.0.as_str(), &payload)
            .await?;

        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInvocationCompleted(start.elapsed()),
            std::collections::BTreeMap::from([("EVENT_TYPE".to_string(), "CAST".to_string())]),
        );
        Ok(())
    }

    async fn process_call_message(
        &mut self,
        source_id: edgeless_api::function_instance::InstanceId,
        target_port: edgeless_api::function_instance::PortId,
        payload: String,
        channel_id: u64,
    ) -> Result<(), super::FunctionInstanceError> {
        let start = tokio::time::Instant::now();

        let res = self
            .function_instance
            .as_mut()
            .ok_or(super::FunctionInstanceError::InternalError)?
            .call(&source_id, target_port.0.as_str(), &payload)
            .await?;

        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInvocationCompleted(start.elapsed()),
            std::collections::BTreeMap::from([("EVENT_TYPE".to_string(), "CALL".to_string())]),
        );

        let mut wh = self.data_plane.clone();
        wh.reply(source_id, channel_id, res).await;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), super::FunctionInstanceError> {
        let start = tokio::time::Instant::now();

        self.function_instance
            .as_mut()
            .ok_or(super::FunctionInstanceError::InternalError)?
            .stop()
            .await?;

        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionStop(start.elapsed()),
            std::collections::BTreeMap::new(),
        );

        Ok(())
    }

    async fn exit(&mut self, exit_status: Result<(), super::FunctionInstanceError>) {
        self.runtime_api
            .send(super::runtime::RuntimeRequest::FunctionExit(
                self.instance_id.clone(),
                exit_status.clone(),
            ))
            .await
            .unwrap_or_else(|_| log::error!("FunctionInstance outlived runner."));

        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionExit(match exit_status {
                Ok(_) => edgeless_telemetry::telemetry_events::FunctionExitStatus::Ok,
                Err(exit_err) => match exit_err {
                    FunctionInstanceError::BadCode => edgeless_telemetry::telemetry_events::FunctionExitStatus::CodeError,
                    _ => edgeless_telemetry::telemetry_events::FunctionExitStatus::InternalError,
                },
            }),
            std::collections::BTreeMap::new(),
        );
    }
}
