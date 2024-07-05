// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
#[derive(Clone)]
pub struct EmbeddedAgent {
    own_node_id: edgeless_api_core::instance_id::NodeId,
    upstream_sender: embassy_sync::channel::Sender<'static, embassy_sync::blocking_mutex::raw::NoopRawMutex, AgentEvent, 2>,
    upstream_receiver: Option<embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::NoopRawMutex, AgentEvent, 2>>,
    inner: &'static core::cell::RefCell<
        embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, &'static mut [&'static mut dyn crate::resource::ResourceDyn]>,
    >,
    orchestrator_url: &'static str,
    registration_signal: &'static embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::NoopRawMutex, RegistrationReply>,
}

pub enum AgentEvent {
    Invocation(edgeless_api_core::invocation::Event<heapless::Vec<u8, 1500>>),
    Registration(
        (
            edgeless_api_core::node_registration::EncodedNodeRegistration<'static>,
            &'static embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::NoopRawMutex, RegistrationReply>,
        ),
    ),
}

pub enum RegistrationReply {
    Sucess,
    Failure,
}

impl EmbeddedAgent {
    pub async fn new(
        spawner: embassy_executor::Spawner,
        node_id: edgeless_api_core::instance_id::NodeId,
        resources: &'static mut [&'static mut dyn crate::resource::ResourceDyn],
        orchestrator_url: &'static str,
    ) -> &'static mut EmbeddedAgent {
        static CHANNEL_RAW: static_cell::StaticCell<embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::NoopRawMutex, AgentEvent, 2>> =
            static_cell::StaticCell::new();
        let channel =
            CHANNEL_RAW.init_with(|| embassy_sync::channel::Channel::<embassy_sync::blocking_mutex::raw::NoopRawMutex, AgentEvent, 2>::new());

        let sender = channel.sender();
        let receiver = channel.receiver();

        static SLF_INNER_RAW: static_cell::StaticCell<
            core::cell::RefCell<
                embassy_sync::mutex::Mutex<
                    embassy_sync::blocking_mutex::raw::NoopRawMutex,
                    &'static mut [&'static mut dyn crate::resource::ResourceDyn],
                >,
            >,
        > = static_cell::StaticCell::new();
        let slf_inner = SLF_INNER_RAW.init_with(|| core::cell::RefCell::new(embassy_sync::mutex::Mutex::new(&mut resources[..])));

        static REPLY_CHANNEL: static_cell::StaticCell<
            embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::NoopRawMutex, RegistrationReply>,
        > = static_cell::StaticCell::new();

        static SLF_RAW: static_cell::StaticCell<EmbeddedAgent> = static_cell::StaticCell::new();
        let slf = SLF_RAW.init_with(|| EmbeddedAgent {
            own_node_id: node_id.clone(),
            upstream_sender: sender,
            upstream_receiver: Some(receiver),
            inner: slf_inner,
            orchestrator_url: orchestrator_url,
            registration_signal: REPLY_CHANNEL
                .init_with(|| embassy_sync::signal::Signal::<embassy_sync::blocking_mutex::raw::NoopRawMutex, RegistrationReply>::new()),
        });

        {
            let inner = slf.inner.borrow_mut();
            let mut lck = inner.lock().await;
            for r in lck.iter_mut() {
                r.launch(spawner, slf.dataplane_handle()).await;
            }
        }

        slf
    }

    pub fn dataplane_handle(&mut self) -> crate::dataplane::EmbeddedDataplaneHandle {
        crate::dataplane::EmbeddedDataplaneHandle { reg: self.clone() }
    }

    pub fn upstream_receiver(
        &mut self,
    ) -> Option<embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::NoopRawMutex, AgentEvent, 2>> {
        self.upstream_receiver.take()
    }

    pub async fn register(&mut self) {
        // let agent_url = "coap://192.168.101.1:7050";
        // let invocation_url = "coap://192.168.101.1:7050";
        let agent_url = "coap://192.168.2.233:7050";
        let invocation_url = "coap://192.168.2.233:7050";

        let tmp = self.inner.borrow_mut();
        let lck = tmp.lock().await;
        let mut resources = heapless::Vec::new();
        for i in &lck[..] {
            let mut outputs = heapless::Vec::new();

            for j in i.outputs() {
                if outputs.push(*j).is_err() {
                    log::error!("Resource has too many outputs!");
                }
            }

            if resources
                .push(edgeless_api_core::node_registration::ResourceProviderSpecification {
                    provider_id: i.provider_id(),
                    class_type: i.resource_class(),
                    outputs: outputs,
                })
                .is_err()
            {
                log::error!("Node has to many resources!");
            }
        }

        let reg = edgeless_api_core::node_registration::EncodedNodeRegistration {
            node_id: edgeless_api_core::node_registration::NodeId(self.own_node_id),
            agent_url: agent_url,
            invocation_url: invocation_url,
            resources: resources,
        };

        loop {
            self.registration_signal.reset();
            self.upstream_sender
                .send(AgentEvent::Registration((reg.clone(), self.registration_signal)))
                .await;
            match self.registration_signal.wait().await {
                RegistrationReply::Sucess => {
                    return;
                }
                _ => {}
            }
        }
    }
}

impl crate::invocation::InvocationAPI for EmbeddedAgent {
    async fn handle(
        &mut self,
        event: edgeless_api_core::invocation::Event<&[u8]>,
    ) -> Result<edgeless_api_core::invocation::LinkProcessingResult, ()> {
        if event.target.node_id != self.own_node_id && event.source.node_id == self.own_node_id {
            let new_event: edgeless_api_core::invocation::Event<heapless::Vec<u8, 1500>> =
                edgeless_api_core::invocation::Event::<heapless::Vec<u8, 1500>> {
                    target: event.target,
                    source: event.source,
                    stream_id: event.stream_id,
                    data: match event.data {
                        edgeless_api_core::invocation::EventData::Cast(val) => {
                            edgeless_api_core::invocation::EventData::Cast(heapless::Vec::<u8, 1500>::from_slice(val).unwrap())
                        }
                        edgeless_api_core::invocation::EventData::Call(val) => {
                            edgeless_api_core::invocation::EventData::Call(heapless::Vec::<u8, 1500>::from_slice(val).unwrap())
                        }
                        edgeless_api_core::invocation::EventData::CallRet(val) => {
                            edgeless_api_core::invocation::EventData::CallRet(heapless::Vec::<u8, 1500>::from_slice(val).unwrap())
                        }
                        edgeless_api_core::invocation::EventData::CallNoRet => edgeless_api_core::invocation::EventData::CallNoRet,
                        edgeless_api_core::invocation::EventData::Err => edgeless_api_core::invocation::EventData::Err,
                    },
                };
            self.upstream_sender.send(AgentEvent::Invocation(new_event)).await;
            Ok(edgeless_api_core::invocation::LinkProcessingResult::FINAL)
        } else {
            let inner = self.inner.borrow_mut();
            let mut lck = inner.lock().await;

            for r in lck.iter_mut() {
                if r.has_instance(&event.target).await {
                    return r.handle(event).await;
                }
            }
            Ok(edgeless_api_core::invocation::LinkProcessingResult::PASSED)
        }
    }
}

impl crate::resource_configuration::ResourceConfigurationAPI for EmbeddedAgent {
    async fn stop(&mut self, resource_id: edgeless_api_core::instance_id::InstanceId) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        let inner = self.inner.borrow_mut();
        let mut lck = inner.lock().await;
        for r in lck.iter_mut() {
            if r.has_instance(&resource_id).await {
                return r.stop(resource_id).await;
            }
        }
        Err(edgeless_api_core::common::ErrorResponse {
            summary: "ResourceProvider Not Found",
            detail: None,
        })
    }

    async fn start<'a>(
        &mut self,
        instance_specification: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<edgeless_api_core::instance_id::InstanceId, edgeless_api_core::common::ErrorResponse> {
        let inner = self.inner.borrow_mut();
        let mut lck = inner.lock().await;
        for r in lck.iter_mut() {
            if r.resource_class() == instance_specification.class_type {
                return r.start(instance_specification).await;
            }
        }
        Err(edgeless_api_core::common::ErrorResponse {
            summary: "ResourceProvider Not Found",
            detail: None,
        })
    }

    async fn patch<'a>(
        &mut self,
        patch_req: edgeless_api_core::resource_configuration::EncodedPatchRequest<'a>,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        let inner = self.inner.borrow_mut();
        let mut lck = inner.lock().await;
        let mut my_patch = patch_req;

        my_patch.instance_id = edgeless_api_core::instance_id::InstanceId {
            node_id: self.own_node_id,
            function_id: my_patch.instance_id.function_id,
        };
        for r in lck.iter_mut() {
            if r.has_instance(&my_patch.instance_id).await {
                return r.patch(my_patch).await;
            }
        }
        Err(edgeless_api_core::common::ErrorResponse {
            summary: "ResourceProvider Not Found",
            detail: None,
        })
    }
}
