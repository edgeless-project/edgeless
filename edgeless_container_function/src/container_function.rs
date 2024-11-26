// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_api::api::container_runtime::ContainerRuntimeAPI;
use futures::{Future, SinkExt, StreamExt};

pub struct ContainerFunction {
    sender: futures::channel::mpsc::UnboundedSender<ContainerFunctionRequest>,
}

enum ContainerFunctionRequest {
    Boot(edgeless_api::guest_api_function::BootData),
    Init(edgeless_api::guest_api_function::FunctionInstanceInit),
    Cast(edgeless_api::guest_api_function::InputEventData),
    Call(
        edgeless_api::guest_api_function::InputEventData,
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::guest_api_function::CallReturn>>,
    ),
    Stop(),
}

enum FiniteStateMachine {
    PreBoot,
    Booted,
    Initialized,
    Stopped,
    Error,
}

impl ContainerFunction {
    pub fn new() -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        log::debug!("new container function created");
        let (sender, receiver) = futures::channel::mpsc::unbounded();

        let main_task = Box::pin(async move {
            Self::main_task(receiver).await;
        });

        (Self { sender }, main_task)
    }

    async fn main_task(receiver: futures::channel::mpsc::UnboundedReceiver<ContainerFunctionRequest>) {
        let mut receiver = receiver;

        // Initialized in BOOT
        let mut host_client;
        let mut host_client_api = None;

        // InstanceID assigned to this function, initialized in BOOT
        let mut instance_id = None;

        // Finite state machine of the function
        let mut fsm = FiniteStateMachine::PreBoot;

        // Main loop that reacts to messages on the receiver channel
        while let Some(req) = receiver.next().await {
            match req {
                ContainerFunctionRequest::Boot(boot_data) => {
                    log::info!(
                        "boot, remote node URL {}, instance_id {}",
                        boot_data.guest_api_host_endpoint,
                        boot_data.instance_id
                    );
                    if std::mem::discriminant(&fsm) != std::mem::discriminant(&FiniteStateMachine::PreBoot) {
                        log::error!("received boot command while not in a pre-boot state: ignored");
                    } else {
                        match edgeless_api::grpc_impl::container_runtime::ContainerRuntimeAPIClient::new(
                            boot_data.guest_api_host_endpoint.as_str(),
                            None,
                        )
                        .await
                        {
                            Ok(client) => {
                                host_client = Some(client);
                                host_client_api = Some(host_client.unwrap().guest_api_host());
                                instance_id = Some(boot_data.instance_id);
                                fsm = FiniteStateMachine::Booted;
                            }
                            Err(err) => {
                                log::error!("container function boot error: {}", err);
                                fsm = FiniteStateMachine::Error;
                            }
                        }
                    }
                }
                ContainerFunctionRequest::Init(init_data) => {
                    log::info!(
                        "init, init_data {}, serialized_state {} bytes",
                        init_data.init_payload,
                        init_data.serialized_state.len()
                    );
                    if std::mem::discriminant(&fsm) != std::mem::discriminant(&FiniteStateMachine::Booted) {
                        log::error!("received init command while not in a booted state: ignored");
                    } else {
                        // Add init logic here.
                        fsm = FiniteStateMachine::Initialized;
                    }
                }
                ContainerFunctionRequest::Cast(event) => {
                    log::info!("cast, src {}, msg {} bytes", event.src, event.msg.len());
                    if std::mem::discriminant(&fsm) != std::mem::discriminant(&FiniteStateMachine::Initialized) {
                        log::error!("received cast command while not in an initialized state: ignored");
                    } else {
                        // Add cast logic here.

                        // This is just an example: the application waits for 1 second then forwards the
                        // event to its output channel called "output".
                        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                        if let Some(ref mut host_client_api) = host_client_api {
                            if let Err(err) = host_client_api
                                .cast(edgeless_api::guest_api_host::OutputEventData {
                                    originator: instance_id.unwrap(),
                                    alias: "output".to_string(),
                                    msg: event.msg,
                                })
                                .await
                            {
                                fsm = FiniteStateMachine::Error;
                                log::error!("error when casting an event to alias \"output\": {}", err);
                            } else {
                                log::info!("event recasted successfully to \"output\"");
                            }
                        }
                    }
                }
                ContainerFunctionRequest::Call(event, reply_sender) => {
                    log::info!("call, src {}, msg {} bytes", event.src, event.msg.len());
                    let res = match std::mem::discriminant(&fsm) == std::mem::discriminant(&FiniteStateMachine::Initialized) {
                        false => {
                            log::error!("received call command while not in an initialized state: ignored");
                            edgeless_api::guest_api_function::CallReturn::Err
                        }
                        true => {
                            // Add call logic here.
                            edgeless_api::guest_api_function::CallReturn::Reply(event.msg)
                        }
                    };
                    match reply_sender.send(Ok(res)) {
                        Ok(_) => {}
                        Err(err) => {
                            fsm = FiniteStateMachine::Error;
                            log::error!("internal communication error on call command: {:?}", err);
                        }
                    }
                }
                ContainerFunctionRequest::Stop() => {
                    log::info!("stop");
                    if std::mem::discriminant(&fsm) != std::mem::discriminant(&FiniteStateMachine::Initialized) {
                        log::error!("received stop command while not in an initialized state: ignored");
                    } else {
                        // Add stop logic here.
                        fsm = FiniteStateMachine::Stopped;
                    }
                }
            }
        }
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::api::container_function::ContainerFunctionAPI + Send> {
        Box::new(ContainerFunctionClient {
            container_function_client: Box::new(GuestAPIFunctionClient { sender: self.sender.clone() }),
        })
    }
}

pub struct ContainerFunctionClient {
    container_function_client: Box<dyn edgeless_api::guest_api_function::GuestAPIFunction>,
}

impl edgeless_api::api::container_function::ContainerFunctionAPI for ContainerFunctionClient {
    fn guest_api_function(&mut self) -> Box<dyn edgeless_api::guest_api_function::GuestAPIFunction> {
        self.container_function_client.clone()
    }
}

#[derive(Clone)]
pub struct GuestAPIFunctionClient {
    sender: futures::channel::mpsc::UnboundedSender<ContainerFunctionRequest>,
}

#[async_trait::async_trait]
impl edgeless_api::guest_api_function::GuestAPIFunction for GuestAPIFunctionClient {
    async fn boot(&mut self, boot_data: edgeless_api::guest_api_function::BootData) -> anyhow::Result<()> {
        match self.sender.send(ContainerFunctionRequest::Boot(boot_data.clone())).await {
            Ok(_) => Ok(()),
            Err(err) => return Err(anyhow::anyhow!("GuestAPIFunction::boot channel error: {}", err)),
        }
    }

    async fn init(&mut self, init_data: edgeless_api::guest_api_function::FunctionInstanceInit) -> anyhow::Result<()> {
        match self.sender.send(ContainerFunctionRequest::Init(init_data.clone())).await {
            Ok(_) => Ok(()),
            Err(err) => return Err(anyhow::anyhow!("GuestAPIFunction::init channel error: {}", err)),
        }
    }

    async fn cast(&mut self, event: edgeless_api::guest_api_function::InputEventData) -> anyhow::Result<()> {
        match self.sender.send(ContainerFunctionRequest::Cast(event.clone())).await {
            Ok(_) => Ok(()),
            Err(err) => return Err(anyhow::anyhow!("GuestAPIFunction::cast channel error: {}", err)),
        }
    }

    async fn call(
        &mut self,
        event: edgeless_api::guest_api_function::InputEventData,
    ) -> anyhow::Result<edgeless_api::guest_api_function::CallReturn> {
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::guest_api_function::CallReturn>>();
        match self.sender.send(ContainerFunctionRequest::Call(event.clone(), reply_sender)).await {
            Ok(_) => match reply_receiver.await {
                Ok(ret) => ret,
                Err(err) => Err(anyhow::anyhow!("GuestAPIFunction::call error: {}", err)),
            },
            Err(err) => return Err(anyhow::anyhow!("GuestAPIFunction::call channel error: {}", err)),
        }
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        match self.sender.send(ContainerFunctionRequest::Stop()).await {
            Ok(_) => Ok(()),
            Err(err) => return Err(anyhow::anyhow!("GuestAPIFunction::stop channel error: {}", err)),
        }
    }
}
