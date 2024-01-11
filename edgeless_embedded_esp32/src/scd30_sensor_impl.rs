// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
pub struct SCD30SensorWrapper<Conn: sensor_scd30::base::Base<Err, Delay>, Delay: embedded_hal::delay::DelayUs, Err: core::fmt::Debug> {
    pub sensor: sensor_scd30::Scd30<Conn, Delay, Err>,
}

impl<Conn: sensor_scd30::base::Base<Err, Delay>, Delay: embedded_hal::delay::DelayUs, Err: core::fmt::Debug>
    edgeless_embedded::resource::scd30_sensor::Sensor for SCD30SensorWrapper<Conn, Delay, Err>
{
    fn init(&mut self, _delay_s: u8) {
        self.sensor.set_measurement_interval(5).unwrap();
        self.sensor.start_continuous(0).unwrap();
    }
    fn read(&mut self) -> Result<edgeless_embedded::resource::scd30_sensor::Measurement, ()> {
        match self.sensor.read_data() {
            Ok(val) => {
                let wrapped_measurement = edgeless_embedded::resource::scd30_sensor::Measurement {
                    co2: val.co2,
                    rh: val.rh,
                    temp: val.temp,
                };
                Ok(wrapped_measurement)
            }
            Err(_) => Err(()),
        }
    }
}
