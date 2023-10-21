#[derive(Clone)]
pub struct EmbeddedAgent {
    own_node_id: edgeless_api_core::instance_id::NodeId,
    upstream_sender: embassy_sync::channel::Sender<
        'static,
        embassy_sync::blocking_mutex::raw::NoopRawMutex,
        edgeless_api_core::invocation::Event<heapless::String<1500>>,
        2,
    >,
    upstream_receiver: Option<
        embassy_sync::channel::Receiver<
            'static,
            embassy_sync::blocking_mutex::raw::NoopRawMutex,
            edgeless_api_core::invocation::Event<heapless::String<1500>>,
            2,
        >,
    >,
    inner: &'static core::cell::RefCell<
        embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, &'static mut [&'static mut dyn crate::resource::ResourceDyn]>,
    >,
}

impl EmbeddedAgent {
    pub async fn new(
        spawner: embassy_executor::Spawner,
        node_id: edgeless_api_core::instance_id::NodeId,
        resources: &'static mut [&'static mut dyn crate::resource::ResourceDyn],
    ) -> &'static mut EmbeddedAgent {
        let channel = static_cell::make_static!(embassy_sync::channel::Channel::<
            embassy_sync::blocking_mutex::raw::NoopRawMutex,
            edgeless_api_core::invocation::Event<heapless::String<1500>>,
            2,
        >::new());
        let sender = channel.sender();
        let receiver = channel.receiver();

        let slf = static_cell::make_static!(EmbeddedAgent {
            own_node_id: node_id.clone(),
            upstream_sender: sender,
            upstream_receiver: Some(receiver),
            inner: static_cell::make_static!(core::cell::RefCell::new(embassy_sync::mutex::Mutex::new(&mut resources[..])))
        });

        {
            let inner = slf.inner.borrow_mut();
            let mut lck = inner.lock().await;
            for r in lck.iter_mut() {
                r.launch(spawner, slf.dataplane_handle());
            }
        }

        slf
    }

    pub fn dataplane_handle(&mut self) -> crate::dataplane::EmbeddedDataplaneHandle {
        crate::dataplane::EmbeddedDataplaneHandle { reg: self.clone() }
    }

    pub fn upstream_receiver(
        &mut self,
    ) -> Option<
        embassy_sync::channel::Receiver<
            'static,
            embassy_sync::blocking_mutex::raw::NoopRawMutex,
            edgeless_api_core::invocation::Event<heapless::String<1500>>,
            2,
        >,
    > {
        self.upstream_receiver.take()
    }
}

impl crate::invocation::InvocationAPI for EmbeddedAgent {
    async fn handle(
        &mut self,
        event: edgeless_api_core::invocation::Event<&[u8]>,
    ) -> Result<edgeless_api_core::invocation::LinkProcessingResult, ()> {
        if event.target.node_id != self.own_node_id && event.source.node_id == self.own_node_id {
            let new_event: edgeless_api_core::invocation::Event<heapless::String<1500>> =
                edgeless_api_core::invocation::Event::<heapless::String<1500>> {
                    target: event.target,
                    source: event.source,
                    stream_id: event.stream_id,
                    data: match event.data {
                        edgeless_api_core::invocation::EventData::Cast(val) => {
                            edgeless_api_core::invocation::EventData::Cast(heapless::String::<1500>::from(core::str::from_utf8(val).unwrap()))
                        }
                        edgeless_api_core::invocation::EventData::Call(val) => {
                            edgeless_api_core::invocation::EventData::Call(heapless::String::<1500>::from(core::str::from_utf8(val).unwrap()))
                        }
                        edgeless_api_core::invocation::EventData::CallRet(val) => {
                            edgeless_api_core::invocation::EventData::CallRet(heapless::String::<1500>::from(core::str::from_utf8(val).unwrap()))
                        }
                        edgeless_api_core::invocation::EventData::CallNoRet => edgeless_api_core::invocation::EventData::CallNoRet,
                        edgeless_api_core::invocation::EventData::Err => edgeless_api_core::invocation::EventData::Err,
                    },
                };
            self.upstream_sender.send(new_event).await;
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
    async fn stop(&mut self, resource_id: edgeless_api_core::instance_id::InstanceId) -> Result<(), ()> {
        let inner = self.inner.borrow_mut();
        let mut lck = inner.lock().await;
        for r in lck.iter_mut() {
            if r.has_instance(&resource_id).await {
                return r.stop(resource_id).await;
            }
        }
        Err(())
    }

    async fn start<'a>(
        &mut self,
        instance_specification: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<edgeless_api_core::instance_id::InstanceId, ()> {
        let inner = self.inner.borrow_mut();
        let mut lck = inner.lock().await;
        for r in lck.iter_mut() {
            if r.provider_id() == instance_specification.provider_id {
                return r.start(instance_specification).await;
            }
        }
        Err(())
    }
}
