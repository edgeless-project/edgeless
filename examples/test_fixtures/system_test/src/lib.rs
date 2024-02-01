// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_function::api::*;

struct SystemTest;

impl Edgefunction for SystemTest {
    fn handle_cast(_src: InstanceId, encoded_message: String) {
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

    fn handle_init(_payload: String, _serialized_state: Option<String>) {}

    fn handle_stop() {}
}

edgeless_function::export!(SystemTest);
