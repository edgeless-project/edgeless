// SPDX-FileCopyrightText: © 2023 TUM
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::api::*;

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

impl Edgefunction for TestFun {
    fn handle_cast(_src: InstanceId, encoded_message: String) {
        log::info!("Resource Processor: 'Cast' called, MSG: {}", encoded_message);
        let values: Vec<_> = encoded_message.split(";").collect();
        if values.len() == 3 {
            let co2: f32 = values[0].parse().unwrap();
            let item = format!("CO2:\n{:.0}", co2);
            cast("check_display", &item);
        }
    }

    fn handle_call(_src: InstanceId, encoded_message: String) -> CallRet {
        log::info!("Resource Processor: 'Call' called, MSG: {}", encoded_message);
        CallRet::Noreply
    }

    fn handle_init(_payload: String, serialized_state: Option<String>) {
        edgeless_function::init_logger();
        log::info!("Resource Processor: 'Init' called");
    }

    fn handle_stop() {
        log::info!("Resource Processor: 'Stop' called");
    }
}

edgeless_function::export!(TestFun);
