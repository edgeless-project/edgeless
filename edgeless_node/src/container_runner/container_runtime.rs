// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use futures::{Future, SinkExt, StreamExt};

pub struct ContainerRuntime {
    sender: futures::channel::mpsc::UnboundedSender<ContainerRuntimeRequest>,
    guest_api_hosts: std::collections::HashMap<edgeless_api::function_instance::InstanceId, crate::base_runtime::guest_api::GuestAPIHost>,
    configuration: std::collections::HashMap<String, String>,
}

enum ContainerRuntimeRequest {
    CAST(edgeless_api::guest_api_host::OutputEventData),
    CASTRAW(edgeless_api::guest_api_host::OutputEventDataRaw),
    CALL(
        edgeless_api::guest_api_host::OutputEventData,
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::guest_api_function::CallReturn>>,
    ),
    CALLRAW(
        edgeless_api::guest_api_host::OutputEventDataRaw,
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::guest_api_function::CallReturn>>,
    ),
    TELEMETRYLOG(edgeless_api::guest_api_host::TelemetryLogEvent),
    SLF(tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::function_instance::InstanceId>>),
    DELAYEDCAST(edgeless_api::guest_api_host::DelayedEventData),
    SYNC(edgeless_api::guest_api_host::SyncData),
}

impl crate::base_runtime::runtime::GuestAPIHostRegister for ContainerRuntime {
    fn needs_to_register(&mut self) -> bool {
        true
    }
    fn register_guest_api_host(
        &mut self,
        instance_id: &edgeless_api::function_instance::InstanceId,
        guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
    ) {
        if let Some(_) = self.guest_api_hosts.insert(*instance_id, guest_api_host) {
            log::warn!("ContainerRuntime: overwrote container function: {}", instance_id);
        }
    }

    fn deregister_guest_api_host(&mut self, instance_id: &edgeless_api::function_instance::InstanceId) {
        if let None = self.guest_api_hosts.remove(&instance_id) {
            log::warn!("ContainerRunTime: trying to deregister non-existing container function {}", instance_id);
        }
    }

    fn configuration(&mut self) -> std::collections::HashMap<String, String> {
        self.configuration.clone()
    }
}

impl ContainerRuntime {
    pub fn new(
        configuration: std::collections::HashMap<String, String>,
    ) -> (
        std::sync::Arc<std::sync::Mutex<Box<dyn crate::base_runtime::runtime::GuestAPIHostRegister + Send>>>,
        std::pin::Pin<Box<dyn Future<Output = ()> + Send>>,
        Box<dyn edgeless_api::container_runtime::ContainerRuntimeAPI + Send>,
    ) {
        log::debug!("new container runtime created");
        let (sender, receiver) = futures::channel::mpsc::unbounded();

        let main_task = Box::pin(async move {
            Self::main_task(receiver).await;
        });

        let runtime_api = Box::new(ContainerRuntimeClient {
            container_runtime_client: Box::new(GuestAPIRuntimeClient { sender: sender.clone() }),
        });

        (
            std::sync::Arc::new(std::sync::Mutex::new(Box::new(Self {
                sender,
                guest_api_hosts: std::collections::HashMap::new(),
                configuration,
            }))),
            main_task,
            runtime_api,
        )
    }

    async fn main_task(receiver: futures::channel::mpsc::UnboundedReceiver<ContainerRuntimeRequest>) {
        let mut receiver = receiver;

        // Main loop that reacts to messages on the receiver channel
        while let Some(req) = receiver.next().await {
            match req {
                ContainerRuntimeRequest::CAST(event) => {
                    log::debug!("cast, alias {}, msg {} bytes", event.alias, event.msg.len());
                    log::info!("XXX");
                }
                ContainerRuntimeRequest::CASTRAW(event) => {
                    log::debug!("cast-raw, dst {}, msg {} bytes", event.dst, event.msg.len());
                }
                ContainerRuntimeRequest::CALL(event, reply_sender) => {
                    log::debug!("call, alias {}, msg {} bytes", event.alias, event.msg.len());
                    let res = edgeless_api::guest_api_function::CallReturn::Err;
                    match reply_sender.send(Ok(res)) {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled: {:?}", err);
                        }
                    }
                }
                ContainerRuntimeRequest::CALLRAW(event, reply_sender) => {
                    log::debug!("call-raw, dst {}, msg {} bytes", event.dst, event.msg.len());
                    let res = edgeless_api::guest_api_function::CallReturn::Err;
                    match reply_sender.send(Ok(res)) {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled: {:?}", err);
                        }
                    }
                }
                ContainerRuntimeRequest::TELEMETRYLOG(event) => {
                    log::debug!(
                        "telemetry-log, log-level {:?}, target {}, msg {}",
                        event.log_level,
                        event.target,
                        event.msg
                    );
                }
                ContainerRuntimeRequest::SLF(reply_sender) => {
                    log::debug!("slf");
                    let res = edgeless_api::function_instance::InstanceId::none();
                    match reply_sender.send(Ok(res)) {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled: {:?}", err);
                        }
                    }
                }
                ContainerRuntimeRequest::DELAYEDCAST(event) => {
                    log::debug!(
                        "delayed-cast, delay {}, alias {}, msg {} bytes",
                        event.delay,
                        event.alias,
                        event.msg.len()
                    )
                }
                ContainerRuntimeRequest::SYNC(sync_data) => {
                    log::debug!("sync, serialized-data {} bytes", sync_data.serialized_data.len());
                }
            }
        }
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::container_runtime::ContainerRuntimeAPI + Send> {
        Box::new(ContainerRuntimeClient {
            container_runtime_client: Box::new(GuestAPIRuntimeClient { sender: self.sender.clone() }),
        })
    }
}

pub struct ContainerRuntimeClient {
    container_runtime_client: Box<dyn edgeless_api::guest_api_host::GuestAPIHost>,
}

impl edgeless_api::container_runtime::ContainerRuntimeAPI for ContainerRuntimeClient {
    fn guest_api_host(&mut self) -> Box<dyn edgeless_api::guest_api_host::GuestAPIHost> {
        self.container_runtime_client.clone()
    }
}

#[derive(Clone)]
pub struct GuestAPIRuntimeClient {
    sender: futures::channel::mpsc::UnboundedSender<ContainerRuntimeRequest>,
}

#[async_trait::async_trait]
impl edgeless_api::guest_api_host::GuestAPIHost for GuestAPIRuntimeClient {
    async fn cast(&mut self, event: edgeless_api::guest_api_host::OutputEventData) -> anyhow::Result<()> {
        match self.sender.send(ContainerRuntimeRequest::CAST(event.clone())).await {
            Ok(_) => Ok(()),
            Err(err) => return Err(anyhow::anyhow!("GuestAPIRuntime::cast channel error: {}", err)),
        }
    }
    async fn cast_raw(&mut self, event: edgeless_api::guest_api_host::OutputEventDataRaw) -> anyhow::Result<()> {
        match self.sender.send(ContainerRuntimeRequest::CASTRAW(event.clone())).await {
            Ok(_) => Ok(()),
            Err(err) => return Err(anyhow::anyhow!("GuestAPIRuntime::cast_raw channel error: {}", err)),
        }
    }
    async fn call(&mut self, event: edgeless_api::guest_api_host::OutputEventData) -> anyhow::Result<edgeless_api::guest_api_function::CallReturn> {
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::guest_api_function::CallReturn>>();
        match self.sender.send(ContainerRuntimeRequest::CALL(event.clone(), reply_sender)).await {
            Ok(_) => match reply_receiver.await {
                Ok(ret) => ret,
                Err(err) => Err(anyhow::anyhow!("GuestAPIFunction::call error: {}", err)),
            },
            Err(err) => return Err(anyhow::anyhow!("GuestAPIFunction::call channel error: {}", err)),
        }
    }
    async fn call_raw(
        &mut self,
        event: edgeless_api::guest_api_host::OutputEventDataRaw,
    ) -> anyhow::Result<edgeless_api::guest_api_function::CallReturn> {
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::guest_api_function::CallReturn>>();
        match self.sender.send(ContainerRuntimeRequest::CALLRAW(event.clone(), reply_sender)).await {
            Ok(_) => match reply_receiver.await {
                Ok(ret) => ret,
                Err(err) => Err(anyhow::anyhow!("GuestAPIFunction::call_raw error: {}", err)),
            },
            Err(err) => return Err(anyhow::anyhow!("GuestAPIFunction::call_raw channel error: {}", err)),
        }
    }
    async fn telemetry_log(&mut self, event: edgeless_api::guest_api_host::TelemetryLogEvent) -> anyhow::Result<()> {
        match self.sender.send(ContainerRuntimeRequest::TELEMETRYLOG(event.clone())).await {
            Ok(_) => Ok(()),
            Err(err) => return Err(anyhow::anyhow!("GuestAPIRuntime::telemetry_log channel error: {}", err)),
        }
    }
    async fn slf(&mut self) -> anyhow::Result<edgeless_api::function_instance::InstanceId> {
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::function_instance::InstanceId>>();
        match self.sender.send(ContainerRuntimeRequest::SLF(reply_sender)).await {
            Ok(_) => match reply_receiver.await {
                Ok(ret) => ret,
                Err(err) => Err(anyhow::anyhow!("GuestAPIFunction::slf error: {}", err)),
            },
            Err(err) => return Err(anyhow::anyhow!("GuestAPIFunction::slf channel error: {}", err)),
        }
    }
    async fn delayed_cast(&mut self, event: edgeless_api::guest_api_host::DelayedEventData) -> anyhow::Result<()> {
        match self.sender.send(ContainerRuntimeRequest::DELAYEDCAST(event.clone())).await {
            Ok(_) => Ok(()),
            Err(err) => return Err(anyhow::anyhow!("GuestAPIRuntime::delayed_cast channel error: {}", err)),
        }
    }
    async fn sync(&mut self, sync_data: edgeless_api::guest_api_host::SyncData) -> anyhow::Result<()> {
        match self.sender.send(ContainerRuntimeRequest::SYNC(sync_data.clone())).await {
            Ok(_) => Ok(()),
            Err(err) => return Err(anyhow::anyhow!("GuestAPIRuntime::sync channel error: {}", err)),
        }
    }
}
