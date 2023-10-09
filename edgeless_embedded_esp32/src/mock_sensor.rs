pub struct MockSensorInner {
    pub instance_id: Option<edgeless_api_core::instance_id::InstanceId>,
    pub data_out_id: Option<edgeless_api_core::instance_id::InstanceId>,
    pub delay: u8,
}

pub struct MockSensorConfiguration {
    pub data_out_id: edgeless_api_core::instance_id::InstanceId,
    pub delay_s: u8,
}

pub struct MockSensor {
    pub inner: &'static core::cell::RefCell<embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, MockSensorInner>>,
}

impl MockSensor {}

impl<'a> crate::resource::Resource<'a, MockSensorConfiguration> for MockSensor {
    fn provider_id(&self) -> &'static str {
        return "mock-sensor-1";
    }

    async fn has_instance(&self, instance_id: &edgeless_api_core::instance_id::InstanceId) -> bool {
        let tmp = self.inner.borrow_mut();
        let lck = tmp.lock().await;

        return lck.instance_id == Some(instance_id.clone());
    }
}

#[embassy_executor::task]
pub async fn mock_sensor_task(
    state: &'static core::cell::RefCell<embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, MockSensorInner>>,
    dataplane_handle: crate::dataplane::EmbeddedDataplaneHandle,
) {
    let mut dataplane_handle = dataplane_handle;

    loop {
        let (instance_id, data_out_id, delay) = {
            let tmp = state.borrow_mut();
            let lck = tmp.lock().await;
            (lck.instance_id, lck.data_out_id, lck.delay)
        };
        if let (Some(instance_id), Some(data_out_id)) = (instance_id, data_out_id) {
            log::info!("Sensor send!");
            dataplane_handle.send(instance_id, data_out_id, "10").await;
        }
        embassy_time::Timer::after(embassy_time::Duration::from_secs(delay as u64)).await;
    }
}

impl edgeless_api_core::invocation::InvocationAPI for MockSensor {
    async fn handle(
        &mut self,
        _event: edgeless_api_core::invocation::Event<&[u8]>,
    ) -> Result<edgeless_api_core::invocation::LinkProcessingResult, ()> {
        log::warn!("Sensor received unexpected Event.");
        Ok(edgeless_api_core::invocation::LinkProcessingResult::FINAL)
    }
}

impl<'a> edgeless_api_core::resource_configuration::ResourceConfigurationAPI<'a, MockSensorConfiguration> for MockSensor {
    async fn parse_configuration(
        data: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<MockSensorConfiguration, ()> {
        let mut out_id: Option<edgeless_api_core::instance_id::InstanceId> = None;

        if data.provider_id != "mock-sensor-1" {
            return Err(());
        }

        for output_callback in data.output_callback_definitions {
            if let Some((key, val)) = output_callback {
                if key == "data_out" {
                    out_id = Some(val);
                    break;
                }
            }
        }

        let out_id = match out_id {
            Some(val) => val,
            None => return Err(()),
        };

        let mut delay: u8 = 10;
        for configuration_option in data.configuration {
            if let Some((key, val)) = configuration_option {
                if key == "delay" {
                    if let Ok(new_delay) = val.parse() {
                        delay = new_delay;
                    }
                    break;
                }
            }
        }

        Ok(MockSensorConfiguration {
            data_out_id: out_id,
            delay_s: delay,
        })
    }

    async fn start(&mut self, instance_specification: MockSensorConfiguration) -> Result<edgeless_api_core::instance_id::InstanceId, ()> {
        let tmp = self.inner.borrow_mut();
        let mut lck = tmp.lock().await;

        if let Some(_) = lck.instance_id {
            return Err(());
        }

        let instance_id = edgeless_api_core::instance_id::InstanceId::new(crate::NODE_ID.clone());

        lck.instance_id = Some(instance_id.clone());
        lck.data_out_id = Some(instance_specification.data_out_id);
        lck.delay = instance_specification.delay_s;
        Ok(instance_id)
    }

    async fn stop(&mut self, resource_id: edgeless_api_core::instance_id::InstanceId) -> Result<(), ()> {
        let tmp = self.inner.borrow_mut();
        let mut lck = tmp.lock().await;

        if let Some(instance_id) = lck.instance_id {
            if instance_id == resource_id {
                lck.instance_id = None;
                lck.data_out_id = None;
            }
        }

        Ok(())
    }
}
