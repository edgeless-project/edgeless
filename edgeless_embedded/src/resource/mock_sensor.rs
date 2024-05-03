// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
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

impl MockSensor {
    async fn parse_configuration<'a>(
        data: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<MockSensorConfiguration, edgeless_api_core::common::ErrorResponse> {
        let mut out_id: Option<edgeless_api_core::instance_id::InstanceId> = None;

        if data.class_type != "mock-scd30-sensor-1" {
            return Err(edgeless_api_core::common::ErrorResponse {
                summary: "Wrong Resource ProviderId",
                detail: None,
            });
        }

        for output_callback in data.output_mapping {
            if let Some((key, val)) = output_callback {
                if key == "data_out" {
                    out_id = Some(val);
                    break;
                }
            }
        }

        let out_id = match out_id {
            Some(val) => val,
            None => {
                return Err(edgeless_api_core::common::ErrorResponse {
                    summary: "Output Configuration Missing",
                    detail: None,
                })
            }
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

    pub async fn new() -> &'static mut dyn crate::resource::ResourceDyn {
        let mock_sensor_state = static_cell::make_static!(core::cell::RefCell::new(embassy_sync::mutex::Mutex::new(MockSensorInner {
            instance_id: None,
            data_out_id: None,
            delay: 30
        })));
        static_cell::make_static!(MockSensor { inner: mock_sensor_state })
    }
}

impl crate::resource::Resource for MockSensor {
    fn provider_id(&self) -> &'static str {
        return "mock-scd30-sensor-1";
    }

    async fn has_instance(&self, instance_id: &edgeless_api_core::instance_id::InstanceId) -> bool {
        let tmp = self.inner.borrow_mut();
        let lck = tmp.lock().await;

        return lck.instance_id == Some(instance_id.clone());
    }

    async fn launch(&mut self, spawner: embassy_executor::Spawner, dataplane_handle: crate::dataplane::EmbeddedDataplaneHandle) {
        spawner.spawn(mock_sensor_task(self.inner, dataplane_handle)).unwrap();
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
            dataplane_handle.send(instance_id, data_out_id, "800.12345;50.12345;20.12345").await;
        }
        embassy_time::Timer::after(embassy_time::Duration::from_secs(delay as u64)).await;
    }
}

impl crate::invocation::InvocationAPI for MockSensor {
    async fn handle(
        &mut self,
        _event: edgeless_api_core::invocation::Event<&[u8]>,
    ) -> Result<edgeless_api_core::invocation::LinkProcessingResult, ()> {
        log::warn!("Sensor received unexpected Event.");
        Ok(edgeless_api_core::invocation::LinkProcessingResult::FINAL)
    }
}

impl crate::resource_configuration::ResourceConfigurationAPI for MockSensor {
    async fn start<'a>(
        &mut self,
        instance_specification: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<edgeless_api_core::instance_id::InstanceId, edgeless_api_core::common::ErrorResponse> {
        log::info!("Mock Sensor Start");
        let instance_specification = Self::parse_configuration(instance_specification).await?;

        let tmp = self.inner.borrow_mut();
        let mut lck = tmp.lock().await;

        if let Some(_) = lck.instance_id {
            return Err(edgeless_api_core::common::ErrorResponse {
                summary: "Resource Busy",
                detail: None,
            });
        }

        let instance_id = edgeless_api_core::instance_id::InstanceId::new(crate::NODE_ID.clone());

        lck.instance_id = Some(instance_id.clone());
        lck.data_out_id = Some(instance_specification.data_out_id);
        lck.delay = instance_specification.delay_s;
        Ok(instance_id)
    }

    async fn stop(&mut self, resource_id: edgeless_api_core::instance_id::InstanceId) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        log::info!("Mock Sensor Stop");
        let tmp = self.inner.borrow_mut();
        let mut lck = tmp.lock().await;

        if let Some(instance_id) = lck.instance_id {
            if instance_id == resource_id {
                lck.instance_id = None;
                lck.data_out_id = None;
            }
        } else {
            return Err(edgeless_api_core::common::ErrorResponse {
                summary: "Wrong Resource InstanceId",
                detail: None,
            });
        }

        Ok(())
    }
}
