use crate::resource::Resource;

pub struct ResourceRegistryInner {
    pub mock_display: crate::mock_display::MockDisplay,
    pub display: crate::epaper_display::EPaperDisplay,
    pub mock_sensor: crate::mock_sensor::MockSensor,
    pub sensor: crate::scd30_sensor::SCD30Sensor,
}

#[derive(Clone)]
pub struct ResourceRegistry {
    pub own_node_id: edgeless_api_core::instance_id::NodeId,
    pub upstream_sender: embassy_sync::channel::Sender<
        'static,
        embassy_sync::blocking_mutex::raw::NoopRawMutex,
        edgeless_api_core::invocation::Event<heapless::String<1500>>,
        2,
    >,
    pub inner: &'static core::cell::RefCell<embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, ResourceRegistryInner>>,
}

impl ResourceRegistry {}

impl edgeless_api_core::invocation::InvocationAPI for ResourceRegistry {
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
            if lck.mock_display.has_instance(&event.target).await {
                return lck.mock_display.handle(event).await;
            }
            if lck.display.has_instance(&event.target).await {
                return lck.display.handle(event).await;
            }
            if lck.mock_sensor.has_instance(&event.target).await {
                return lck.mock_sensor.handle(event).await;
            }
            if lck.sensor.has_instance(&event.target).await {
                return lck.sensor.handle(event).await;
            }
            Ok(edgeless_api_core::invocation::LinkProcessingResult::PASSED)
        }
    }
}

impl<'a>
    edgeless_api_core::resource_configuration::ResourceConfigurationAPI<
        'a,
        edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    > for ResourceRegistry
{
    async fn parse_configuration(
        data: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>, ()> {
        Ok(data)
    }

    async fn stop(&mut self, resource_id: edgeless_api_core::instance_id::InstanceId) -> Result<(), ()> {
        let inner = self.inner.borrow_mut();
        let mut lck = inner.lock().await;
        if lck.mock_display.has_instance(&resource_id).await {
            lck.mock_display.stop(resource_id);
            return Ok(());
        }
        if lck.display.has_instance(&resource_id).await {
            lck.display.stop(resource_id);
            return Ok(());
        }
        if (lck.mock_sensor.has_instance(&resource_id)).await {
            lck.mock_sensor.stop(resource_id);
            return Ok(());
        }
        if (lck.sensor.has_instance(&resource_id)).await {
            lck.sensor.stop(resource_id);
            return Ok(());
        }
        Err(())
    }

    async fn start(
        &mut self,
        instance_specification: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<edgeless_api_core::instance_id::InstanceId, ()> {
        if let Ok(display_config) = crate::mock_display::MockDisplay::parse_configuration(instance_specification.clone()).await {
            let inner = self.inner.borrow_mut();
            let mut lck = inner.lock().await;
            return lck.mock_display.start(display_config).await;
        }
        if let Ok(display_config) = crate::epaper_display::EPaperDisplay::parse_configuration(instance_specification.clone()).await {
            let inner = self.inner.borrow_mut();
            let mut lck = inner.lock().await;
            return lck.display.start(display_config).await;
        }
        if let Ok(sensor_config) = crate::mock_sensor::MockSensor::parse_configuration(instance_specification.clone()).await {
            let inner = self.inner.borrow_mut();
            let mut lck = inner.lock().await;
            return lck.mock_sensor.start(sensor_config).await;
        }
        if let Ok(sensor_config) = crate::scd30_sensor::SCD30Sensor::parse_configuration(instance_specification).await {
            let inner = self.inner.borrow_mut();
            let mut lck = inner.lock().await;
            return lck.sensor.start(sensor_config).await;
        }
        Err(())
    }
}
