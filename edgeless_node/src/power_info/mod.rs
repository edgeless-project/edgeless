// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use tokio_modbus::prelude::Reader;

/// Query power information via Modbus/TCP.
/// Implementation tested with Raritan PX3-5190NR-M11 PDUs.
pub struct PowerInfo {
    context: Option<tokio_modbus::client::Context>,
    socket_addr: std::net::SocketAddr,
    outlet_number: u16,
}

impl PowerInfo {
    /// Create a new power information object, which can be queried to read
    /// power values for a single outlet..
    pub async fn new(socket_addr: &str, outlet_number: u16) -> anyhow::Result<Self> {
        let socket_addr = socket_addr.parse()?;
        Ok(Self {
            context: None,
            socket_addr,
            outlet_number,
        })
    }

    /// Return the active power, in Watts, of the outlet specified in the ctor.
    /// A negative value means that something went wrong.
    pub async fn active_power(&mut self) -> f32 {
        if self.context.is_none() {
            self.context = tokio_modbus::client::tcp::connect(self.socket_addr).await.ok();
        }

        if let Some(context) = &mut self.context {
            match context.read_holding_registers(PowerInfo::active_power_addr(self.outlet_number), 2).await {
                Ok(res) => match res {
                    Ok(data) => return PowerInfo::parse_f32(&data),
                    Err(_err) => self.context = None,
                },
                Err(_err) => self.context = None,
            };
        }
        -1.0
    }

    /// Convert bytes to an IEEE-754 floating point.
    fn parse_f32(data: &[u16]) -> f32 {
        let bytes: Vec<u8> = data.iter().fold(vec![], |mut x, elem| {
            x.push((elem >> 8) as u8);
            x.push((elem & 0xff) as u8);
            x
        });
        let byte_array: [u8; 4] = bytes[0..4].try_into().expect("Needed 4 bytes for a float");
        f32::from_be_bytes(byte_array)
    }

    /// Return the address of the first holding register for active power.
    fn active_power_addr(outlet_number: u16) -> u16 {
        32786 + (outlet_number - 1) * 256
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_modbus::prelude::Client;

    #[tokio::test]
    #[ignore]
    async fn test_modbus_tcp_raritan_px3() {
        let socket_addr = "127.0.0.1:5502".parse().unwrap();

        println!("Connecting to {}", socket_addr);

        let mut ctx = tokio_modbus::client::tcp::connect(socket_addr).await.unwrap();

        println!("Fetching the value of the float register from a Raritan PDU");
        let data = ctx.read_holding_registers(0x8012, 2).await.unwrap().unwrap();

        let bytes: Vec<u8> = data.iter().fold(vec![], |mut x, elem| {
            x.push((elem >> 8) as u8);
            x.push((elem & 0xff) as u8);
            x
        });
        let byte_array: [u8; 4] = bytes[0..4].try_into().expect("Needed 4 bytes for a float");
        let power: f32 = f32::from_be_bytes(byte_array);
        println!("Power is {} W", power);

        println!("Disconnecting");
        ctx.disconnect().await.unwrap();

        // Query 8 active power values with PowerInfo
        println!("With PowerInfo");
        for outlet_number in 1..=8 {
            let mut power_info = PowerInfo::new("127.0.0.1:5502", outlet_number).await.unwrap();
            println!("#{} {} W", outlet_number, power_info.active_power().await);
        }
    }
}
