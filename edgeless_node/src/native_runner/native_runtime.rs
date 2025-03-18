// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

use futures::{Future, SinkExt, StreamExt};
use std::collections::HashMap;

pub struct NativeRuntime {
    guest_api_hosts: HashMap<edgeless_api::function_instance::InstanceId, crate::base_runtime::guest_api::GuestAPIHost>,
    configuration: HashMap<String, String>,
}

enum NativeRuntimeRequest {
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

impl crate::base_runtime::runtime::GuestAPIHostRegister for NativeRuntime {
    fn needs_to_register(&mut self) -> bool {
        false
    }

    fn register_guest_api_host(
        &mut self, 
        instance_id: &edgeless_api::function_instance::InstanceId,
        guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
    ) {
        if let Some(_) = self.guest_api_hosts.remove(&instance_id) {
            log::warn!("NativeRuntime: overwrote container function: {}", instance_id);
        }
    }

    fn deregister_guest_api_host(&mut self, instance_id: &edgeless_api::function_instance::InstanceId) {
        if let None = self.guest_api_hosts.remove(&instance_id) {
            log::warn!("NativeRuntime: trying to deregister non-existing container function {}", instance_id);
        }
    }

    fn guest_api_host(
        &mut self,
        instance_id: &edgeless_api::function_instance::InstanceId,
    ) -> Option<&mut crate::base_runtime::guest_api::GuestAPIHost> {
        self.guest_api_hosts.get_mut(&instance_id)
    }

    fn configuration(&mut self) -> HashMap<String, String> {
        self.configuration.clone()
    }
}

impl NativeRuntime {

    pub fn new(configuration: HashMap<String, String>) -> (
        std::sync::Arc<tokio::sync::Mutex<Box<dyn crate::base_runtime::runtime::GuestAPIHostRegister + Send>>>,
        std::pin::Pin<Box<dyn Future<Output = ()> + Send>>,
        Box<dyn edgeless_api::native_runtime::NativeRuntimeAPI + Send>,
    ) {
        log::debug!("New native runtime created");
        let (sender, receiver) = futures::channel::mpsc::unbounded::<NativeRuntimeRequest>();

        let native_runtime: std::sync::Arc<tokio::sync::Mutex<Box<dyn crate::base_runtime::runtime::GuestAPIHostRegister + Send>>> = 
            std::sync::Arc::new(tokio::sync::Mutex::new(Box::new(Self {
                guest_api_hosts: HashMap::new(),
                configuration,
            })));

        let native_runtime_clone = native_runtime.clone();
        let main_task = Box::pin(async move {
            Self::main_task(receiver, native_runtime_clone).await;
        });

        (
            native_runtime,
            main_task,
            Box::new(NativeRuntimeClient {
                native_runtime_client: Box::new(GuestAPIRuntimeClient { sender }),
            }),
        )
    }

    async fn main_task(
        receiver: futures::channel::mpsc::UnboundedReceiver<NativeRuntimeRequest>,
        native_runtime: std::sync::Arc<tokio::sync::Mutex<Box<dyn crate::base_runtime::runtime::GuestAPIHostRegister + Send>>>,
    ) {
        let mut receiver = receiver;

        while let Some(req) = receiver.next().await {
            match req {
                NativeRuntimeRequest::CAST(event) => {
                    log::debug!("cast, alias {}, msg {} bytes", event.alias, event.msg.len());
                    if let Some(runtime) = native_runtime.lock().await.guest_api_host(&event.originator) {
                        if let Err(_) = runtime.cast_alias(&event.alias, String::from_utf8(event.msg).unwrap().as_str()).await {
                            log::error!("error occured when casting an event towards alias {}: droppped", event.alias);
                        }
                    } else {
                        log::warn!(
                            "no function instance with matching ID {} when casting an event towards alias {}: dropped",
                            event.originator,
                            event.alias,
                        );
                    }
                }
                NativeRuntimeRequest::CASTRAW(event) => {}
                NativeRuntimeRequest::CALL(event, reply_sender) => {}
                NativeRuntimeRequest::CALLRAW(event, reply_sender) => {}
                NativeRuntimeRequest::TELEMETRYLOG(event) => {
                    log::debug!(
                        "telemetry-log, log-level {:?}, target {}, msg {}",
                        event.log_level,
                        event.target,
                        event.msg
                    );
                    if let Some(runtime) = native_runtime.lock().await.guest_api_host(&event.originator) {
                        runtime.telemetry_log (
                            match event.log_level {
                                edgeless_api::guest_api_host::TelemetryLogLevel::Error => {
                                    edgeless_telemetry::telemetry_events::TelemetryLogLevel::Error
                                }
                                edgeless_api::guest_api_host::TelemetryLogLevel::Warn => {
                                    edgeless_telemetry::telemetry_events::TelemetryLogLevel::Warn
                                }
                                edgeless_api::guest_api_host::TelemetryLogLevel::Info => {
                                    edgeless_telemetry::telemetry_events::TelemetryLogLevel::Info
                                }
                                edgeless_api::guest_api_host::TelemetryLogLevel::Debug => {
                                    edgeless_telemetry::telemetry_events::TelemetryLogLevel::Debug
                                }
                                edgeless_api::guest_api_host::TelemetryLogLevel::Trace => {
                                    edgeless_telemetry::telemetry_events::TelemetryLogLevel::Trace
                                }
                            },
                            &event.target,
                            &event.msg,
                        ).await;
                    } else {
                        log::warn!(
                            "no function instance with matching ID {} when issuing a telemtry_log with target {}: ignored",
                            event.originator,
                            event.target
                        );
                    }
                }
                NativeRuntimeRequest::SLF(reply_snder) => {}
                NativeRuntimeRequest::DELAYEDCAST(event) => {}
                NativeRuntimeRequest::SYNC(sync_data) => {}                
            }
        }
    }
}

pub struct NativeRuntimeClient {
    native_runtime_client: Box<dyn edgeless_api::guest_api_host::GuestAPIHost>,
}

impl NativeRuntimeClient {
    pub fn new(native_runtime_client: Box<dyn edgeless_api::guest_api_host::GuestAPIHost>) -> Self {
        Self {
            native_runtime_client: native_runtime_client,
        }
    }
}

impl edgeless_api::native_runtime::NativeRuntimeAPI for NativeRuntimeClient {
    fn guest_api_host(&mut self) -> Box<dyn edgeless_api::guest_api_host::GuestAPIHost> {
        self.native_runtime_client.clone()
    }
    
    /*#[no_mangle] 
    unsafe extern "C" fn telemetry_log_asm (
        &mut self,
        level: usize, 
        target_ptr: *const u8, 
        target_len: usize, 
        msg_ptr: *const u8, 
        msg_len: usize,
    ) {
        let target: &str = std::str::from_utf8(core::slice::from_raw_parts(target_ptr, target_len)).unwrap();
        let msg: &str = std::str::from_utf8(core::slice::from_raw_parts(msg_ptr, msg_len)).unwrap();

        println!("Native RT: Log: Target: {} msg: {}", target, msg);

    
        //self.native_runtime_client.telemetry_log(edgeless_telemetry::telemetry_events::TelemetryLogLevel::Info, target, msg);
        //let telemetry_log_event = TelemetryLogEvent(edgeless_api_core::instance_id::InstanceId edgeless_telemetry::telemetry_events::TelemetryLogLevel::Info, target, msg);
        //self.native_runtime_client.telemetry_log(telemetry_log_event);
            
    }*/
}

#[derive(Clone)]
pub struct GuestAPIRuntimeClient {
    sender: futures::channel::mpsc::UnboundedSender<NativeRuntimeRequest>,
}

#[async_trait::async_trait]
impl edgeless_api::guest_api_host::GuestAPIHost for GuestAPIRuntimeClient {
    async fn cast(&mut self, event: edgeless_api::guest_api_host::OutputEventData) -> anyhow::Result<()> {
        match self.sender.send(NativeRuntimeRequest::CAST(event.clone())).await {
            Ok(_) => Ok(()),
            Err(err) => return Err(anyhow::anyhow!("GuestAPIRuntime::cast channel error: {}", err)),
        }
    }
    async fn cast_raw(&mut self, event: edgeless_api::guest_api_host::OutputEventDataRaw) -> anyhow::Result<()> {
        match self.sender.send(NativeRuntimeRequest::CASTRAW(event.clone())).await {
            Ok(_) => Ok(()),
            Err(err) => return Err(anyhow::anyhow!("GuestAPIRuntime::cast_raw channel error: {}", err)),
        }
    }
    async fn call(
        &mut self, 
        event: edgeless_api::guest_api_host::OutputEventData,
    ) -> anyhow::Result<(edgeless_api::guest_api_function::CallReturn)> {
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::guest_api_function::CallReturn>>();
        match self.sender.send(NativeRuntimeRequest::CALL(event.clone(), reply_sender)).await {
            Ok(_) => match reply_receiver.await {
                Ok(ret) => ret,
                Err(err) => Err(anyhow::anyhow!("GuestAPIFunction::call error: {}", err)),
            },
            Err(err) => Err(anyhow::anyhow!("GuestAPIFunction::call channel error: {}", err)),
        }
    }
    async fn call_raw(
        &mut self, 
        event: edgeless_api::guest_api_host::OutputEventDataRaw,
    ) -> anyhow::Result<edgeless_api::guest_api_function::CallReturn> {
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::guest_api_function::CallReturn>>();
        match self.sender.send(NativeRuntimeRequest::CALLRAW(event.clone(), reply_sender)).await {
            Ok(_) => match reply_receiver.await {
                Ok(ret) => ret,
                Err(err) => Err(anyhow::anyhow!("GuestAPIFUnction::call_raw error: {}", err)),
            },
            Err(err) => Err(anyhow::anyhow!("GuestAPIFUnction::call_raw channel error: {}", err)),
        }
    }
    async fn telemetry_log(&mut self, event: edgeless_api::guest_api_host::TelemetryLogEvent) -> anyhow::Result<()> {
        match self.sender.send(NativeRuntimeRequest::TELEMETRYLOG(event.clone())).await {
            Ok(_) => Ok(()),
            Err(err) => return Err(anyhow::anyhow!("GuestAPIRuntime::telemetry_log channel error {}", err)),
        }
 
    }
    async fn slf(&mut self) -> anyhow::Result<edgeless_api::function_instance::InstanceId> {
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::function_instance::InstanceId>>();
        match self.sender.send(NativeRuntimeRequest::SLF(reply_sender)).await {
            Ok(_) => match reply_receiver.await {
                Ok(ret) => ret,
                Err(err) => Err(anyhow::anyhow!("GuestAPIRuntime::slf error: {}", err)),
            }
            Err(err) => Err(anyhow::anyhow!("GuestAPIRuntime::slf channel error: {}", err)),
        }
    }
    async fn delayed_cast(&mut self, event: edgeless_api::guest_api_host::DelayedEventData) -> anyhow::Result<()> {
        match self.sender.send(NativeRuntimeRequest::DELAYEDCAST(event.clone())).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("GuestAPIRuntime::delayed_cast channel error: {}", err)),
        }
    }
    async fn sync(&mut self, sync_data: edgeless_api::guest_api_host::SyncData) -> anyhow::Result<()> {
        match self.sender.send(NativeRuntimeRequest::SYNC(sync_data.clone())).await {
            Ok(_) => Ok(()),
            Err(err) => return Err(anyhow::anyhow!("GuestAPIRuntime::sync channel error {}", err)),
        }
    }
}