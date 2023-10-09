use sensor_scd30::base::Base;

pub trait Sensor {
    fn init(&mut self, delay_s: u8);
    fn read(&mut self) -> Result<sensor_scd30::Measurement, ()>;
}

pub struct SCD30SensorWrapper<Conn: Base<Err, Delay>, Delay: hal::prelude::eh1::_embedded_hal_blocking_delay_DelayMs<u32>, Err: core::fmt::Debug> {
    pub sensor: sensor_scd30::Scd30<Conn, Delay, Err>,
}

impl<Conn: Base<Err, Delay>, Delay: hal::prelude::eh1::_embedded_hal_blocking_delay_DelayMs<u32>, Err: core::fmt::Debug> Sensor
    for SCD30SensorWrapper<Conn, Delay, Err>
{
    fn init(&mut self, _delay_s: u8) {
        self.sensor.set_measurement_interval(5).unwrap();
        self.sensor.start_continuous(0).unwrap();
    }
    fn read(&mut self) -> Result<sensor_scd30::Measurement, ()> {
        match self.sensor.read_data() {
            Ok(val) => Ok(val),
            Err(_) => Err(()),
        }
    }
}

pub struct SCD30SensorInner {
    pub instance_id: Option<edgeless_api_core::instance_id::InstanceId>,
    pub data_out_id: Option<edgeless_api_core::instance_id::InstanceId>,
    pub sensor: &'static mut dyn Sensor,
    pub delay: u8,
}

pub struct SCD30SensorConfiguration {
    pub data_out_id: edgeless_api_core::instance_id::InstanceId,
}

pub struct SCD30Sensor {
    pub inner: &'static core::cell::RefCell<embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, SCD30SensorInner>>,
}

impl SCD30Sensor {}

impl<'a> crate::resource::Resource<'a, SCD30SensorConfiguration> for SCD30Sensor {
    fn provider_id(&self) -> &'static str {
        return "scd30-sensor-1";
    }

    async fn has_instance(&self, instance_id: &edgeless_api_core::instance_id::InstanceId) -> bool {
        let tmp = self.inner.borrow_mut();
        let lck = tmp.lock().await;

        return lck.instance_id == Some(instance_id.clone());
    }
}

#[embassy_executor::task]
pub async fn scd30_sensor_task(
    state: &'static core::cell::RefCell<embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, SCD30SensorInner>>,
    dataplane_handle: crate::dataplane::EmbeddedDataplaneHandle,
) {
    let mut dataplane_handle = dataplane_handle;
    let delay = {
        let tmp = state.borrow_mut();
        let mut lck = tmp.lock().await;
        let delay = lck.delay;
        lck.sensor.init(delay);
        delay
    };

    embassy_time::Timer::after(embassy_time::Duration::from_secs(delay as u64)).await;
    loop {
        let (instance_id, data_out_id, data) = {
            let tmp = state.borrow_mut();
            let mut lck = tmp.lock().await;

            let data = match lck.sensor.read() {
                Ok(val) => {
                    if !val.co2.is_nan() && !val.rh.is_nan() && !val.rh.is_nan() {
                        val
                    } else {
                        continue;
                    }
                }
                Err(_) => {
                    continue;
                }
            };

            (lck.instance_id, lck.data_out_id, data)
        };
        if let (Some(instance_id), Some(data_out_id)) = (instance_id, data_out_id) {
            let mut buffer = heapless::String::<150>::new();
            if core::fmt::write(&mut buffer, format_args!("{:.5};{:.5};{:.5}", data.co2, data.rh, data.temp)).is_ok() {
                dataplane_handle.send(instance_id, data_out_id, buffer.as_str()).await;
            }
        }
        embassy_time::Timer::after(embassy_time::Duration::from_secs(delay as u64)).await;
    }
}

impl edgeless_api_core::invocation::InvocationAPI for SCD30Sensor {
    async fn handle(
        &mut self,
        _event: edgeless_api_core::invocation::Event<&[u8]>,
    ) -> Result<edgeless_api_core::invocation::LinkProcessingResult, ()> {
        log::warn!("SCD30 Sensor received unexpected Event.");
        Ok(edgeless_api_core::invocation::LinkProcessingResult::FINAL)
    }
}

impl<'a> edgeless_api_core::resource_configuration::ResourceConfigurationAPI<'a, SCD30SensorConfiguration> for SCD30Sensor {
    async fn parse_configuration(
        data: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<SCD30SensorConfiguration, ()> {
        let mut out_id: Option<edgeless_api_core::instance_id::InstanceId> = None;

        if data.provider_id != "scd30-sensor-1" {
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

        Ok(SCD30SensorConfiguration { data_out_id: out_id })
    }

    async fn start(&mut self, instance_specification: SCD30SensorConfiguration) -> Result<edgeless_api_core::instance_id::InstanceId, ()> {
        let tmp = self.inner.borrow_mut();
        let mut lck = tmp.lock().await;

        if let Some(_) = lck.instance_id {
            return Err(());
        }

        let instance_id = edgeless_api_core::instance_id::InstanceId::new(crate::NODE_ID.clone());

        lck.instance_id = Some(instance_id.clone());
        lck.data_out_id = Some(instance_specification.data_out_id);
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
