// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_function::api::*;

struct SystemTest;

impl Edgefunction for SystemTest {
    fn handle_cast(_src: InstanceId, encoded_message: String) {
        // log::info!("cast {}: {}", _src.function, encoded_message);
        match encoded_message.parse::<i32>() {
            Ok(val) => {
                cast("out1", format!("{}", val / 2).as_str());
                cast("out2", format!("{}", val - 1).as_str());
            }
            Err(err) => cast("err", format!("parsing error: {}", err).as_str()),
        };
        cast("log", encoded_message.as_str());
    }

    fn handle_call(_src: InstanceId, _encoded_message: String) -> CallRet {
        CallRet::Noreply
    }

    fn handle_init(payload: String, _serialized_state: Option<String>) {
        // edgeless_function::init_logger();
        // log::info!("started: {}", payload);
        if !payload.is_empty() {
            match payload.parse::<i32>() {
                Ok(_) => delayed_cast(500, "self", &payload),
                Err(_) => {}
            }
        }
    }

    fn handle_stop() {}
}

edgeless_function::export!(SystemTest);
