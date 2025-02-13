// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
#[derive(Debug)]
pub struct Measurement {
    pub co2: f32,
    pub rh: f32,
    pub temp: f32,
}

pub struct SCD30SensorInner {
    pub instance_id: Option<edgeless_api_core::instance_id::InstanceId>,
    pub data_out_id: Option<edgeless_api_core::instance_id::InstanceId>,
    pub data_receiver: Option<embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, Measurement, 2>>,
    // pub delay: u8,
}

#[derive(Debug)]
pub struct SensorError;

impl core::fmt::Display for SensorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Sensor error")
    }
}

pub trait Sensor {
    fn init(&mut self, delay_s: u8);
    fn read(&mut self) -> Result<Measurement, SensorError>;
}

pub struct SCD30SensorConfiguration {
    pub data_out_id: Option<edgeless_api_core::instance_id::InstanceId>,
}

pub struct SCD30Sensor {
    pub inner: &'static core::cell::RefCell<embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, SCD30SensorInner>>,
}

impl SCD30Sensor {
    #[allow(clippy::needless_lifetimes)] // not needless
    async fn parse_configuration<'a>(
        data: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<SCD30SensorConfiguration, edgeless_api_core::common::ErrorResponse> {
        let out_id: Option<edgeless_api_core::instance_id::InstanceId> = None;

        if data.class_type != "scd30-sensor" {
            return Err(edgeless_api_core::common::ErrorResponse {
                summary: "Wrong Resource ProviderId",
                detail: None,
            });
        }
        Ok(SCD30SensorConfiguration { data_out_id: out_id })
    }

    #[allow(clippy::new_ret_no_self)]
    pub async fn new(
        data_receiver: embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, Measurement, 2>,
    ) -> &'static mut dyn crate::resource::ResourceDyn {
        static SENSOR_STATE_RAW: static_cell::StaticCell<
            core::cell::RefCell<embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, SCD30SensorInner>>,
        > = static_cell::StaticCell::new();
        let sensor_state = SENSOR_STATE_RAW.init_with(|| {
            core::cell::RefCell::new(embassy_sync::mutex::Mutex::new(SCD30SensorInner {
                instance_id: None,
                data_out_id: None,
                data_receiver: Some(data_receiver),
            }))
        });
        static SLF_RAW: static_cell::StaticCell<SCD30Sensor> = static_cell::StaticCell::new();
        SLF_RAW.init_with(|| SCD30Sensor { inner: sensor_state })
    }
}

#[embassy_executor::task]
pub async fn scd30_reader_task(
    sensor: &'static mut dyn Sensor,
    sender: embassy_sync::channel::Sender<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, Measurement, 2>,
) {
    sensor.init(10);
    embassy_time::Timer::after(embassy_time::Duration::from_secs(10_u64)).await;
    loop {
        let data = {
            match sensor.read() {
                Ok(val) => {
                    if !val.co2.is_nan() && !val.rh.is_nan() && !val.rh.is_nan() {
                        // log::info!("{:?}", val);
                        val
                    } else {
                        continue;
                    }
                }
                Err(_) => {
                    continue;
                }
            }
        };
        sender.send(data).await;
        embassy_time::Timer::after(embassy_time::Duration::from_secs(10_u64)).await;
    }
}

impl crate::resource::Resource for SCD30Sensor {
    fn provider_id(&self) -> &'static str {
        "scd30-sensor-bridge-1"
    }

    fn resource_class(&self) -> &'static str {
        "scd30-sensor"
    }

    fn outputs(&self) -> &'static [&'static str] {
        &["data_out"]
    }

    #[allow(clippy::await_holding_refcell_ref)]
    async fn has_instance(&self, instance_id: &edgeless_api_core::instance_id::InstanceId) -> bool {
        let tmp = self.inner.borrow_mut();
        let lck = tmp.lock().await;

        lck.instance_id == Some(*instance_id)
    }

    async fn launch(&mut self, spawner: embassy_executor::Spawner, dataplane_handle: crate::dataplane::EmbeddedDataplaneHandle) {
        spawner.spawn(scd30_sensor_task(self.inner, dataplane_handle)).unwrap();
    }
}

#[embassy_executor::task]
#[allow(clippy::await_holding_refcell_ref)]
pub async fn scd30_sensor_task(
    state: &'static core::cell::RefCell<embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, SCD30SensorInner>>,
    dataplane_handle: crate::dataplane::EmbeddedDataplaneHandle,
) {
    let mut dataplane_handle = dataplane_handle;

    let receiver = {
        let tmp = state.borrow_mut();
        let mut lck = tmp.lock().await;
        lck.data_receiver.take().unwrap()
    };

    loop {
        let measurement = receiver.receive().await;

        let tmp = state.borrow_mut();
        let lck = tmp.lock().await;

        if let (Some(instance_id), Some(data_out_id)) = (lck.instance_id, lck.data_out_id) {
            let mut buffer = heapless::String::<150>::new();
            if core::fmt::write(
                &mut buffer,
                format_args!("{:.5};{:.5};{:.5}", measurement.co2, measurement.rh, measurement.temp),
            )
            .is_ok()
            {
                dataplane_handle.send(instance_id, data_out_id, buffer.as_str()).await;
            }
        }
    }
}

impl crate::invocation::InvocationAPI for SCD30Sensor {
    async fn handle(
        &mut self,
        _event: edgeless_api_core::invocation::Event<&[u8]>,
    ) -> Result<edgeless_api_core::invocation::LinkProcessingResult, ()> {
        log::warn!("SCD30 Sensor received unexpected Event.");
        Ok(edgeless_api_core::invocation::LinkProcessingResult::FINAL)
    }
}

#[allow(clippy::await_holding_refcell_ref)]
impl crate::resource_configuration::ResourceConfigurationAPI for SCD30Sensor {
    #[allow(clippy::needless_lifetimes)]
    async fn start<'a>(
        &mut self,
        instance_specification: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<edgeless_api_core::instance_id::InstanceId, edgeless_api_core::common::ErrorResponse> {
        let instance_specification = SCD30Sensor::parse_configuration(instance_specification).await?;

        let tmp = self.inner.borrow_mut();
        let mut lck = tmp.lock().await;

        if lck.instance_id.is_some() {
            return Err(edgeless_api_core::common::ErrorResponse {
                summary: "Resource Busy",
                detail: None,
            });
        }

        let instance_id = edgeless_api_core::instance_id::InstanceId::new(crate::NODE_ID);

        lck.instance_id = Some(instance_id);
        lck.data_out_id = instance_specification.data_out_id;
        log::info!("Start Sensor");
        Ok(instance_id)
    }

    async fn stop(&mut self, resource_id: edgeless_api_core::instance_id::InstanceId) -> Result<(), edgeless_api_core::common::ErrorResponse> {
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

    async fn patch(
        &mut self,
        patch_req: edgeless_api_core::resource_configuration::EncodedPatchRequest<'_>,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        let tmp = self.inner.borrow_mut();
        let mut lck = tmp.lock().await;

        for (key, val) in patch_req.output_mapping.into_iter().flatten() {
            if key == "data_out" {
                lck.data_out_id = Some(val);
                break;
            }
        }

        Ok(())
    }
}
