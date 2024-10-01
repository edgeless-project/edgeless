// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct TestFun;

#[derive(minicbor::Decode, minicbor::CborLen)]
struct SCD30Measurement {
    #[n(0)]
    co2: f32,
    #[n(1)]
    rh: f32,
    #[n(2)]
    temp: f32,
}

impl EdgeFunction for TestFun {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        let str_message = core::str::from_utf8(encoded_message).unwrap();
        log::info!("Resource Processor: 'Cast' called, MSG: {}", str_message);
        let values: Vec<_> = str_message.split(";").collect();
        if values.len() == 3 {
            let co2: f32 = values[0].parse().unwrap();
            let item = format!("CO2:\n{:.0}", co2);
            cast("check_display", item.as_bytes());
        }
    }

    fn handle_call(_src: InstanceId, encoded_message: &[u8]) -> CallRet {
        log::info!("Resource Processor: 'Call' called, MSG: {:?}", encoded_message);
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Resource Processor: 'Init' called");
    }

    fn handle_stop() {
        log::info!("Resource Processor: 'Stop' called");
    }
}

edgeless_function::export!(TestFun);
