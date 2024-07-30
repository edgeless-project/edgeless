// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_function::*;

struct SystemTest;

impl EdgeFunction for SystemTest {
    fn handle_cast(_src: InstanceId, port: &str, encoded_message: &[u8]) {
        let str_message = core::str::from_utf8(encoded_message).unwrap();
        log::info!("cast {:?}: {}", _src.component_id, str_message);

        match str_message.parse::<i32>() {
            Ok(val) => {
                cast("out1", format!("{}", val / 2).as_str().as_bytes());
                cast("out2", format!("{}", val - 1).as_str().as_bytes());
            }
            Err(err) => cast("err", format!("parsing error: {}", err).as_str().as_bytes()),
        };
        cast("log", encoded_message);
    }

    fn handle_call(_src: InstanceId, port: &str, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("started: {:?}", payload);
        if let Some(pld) = payload {
            match core::str::from_utf8(pld).unwrap().parse::<i32>() {
                Ok(_) => delayed_cast(1000, "self", pld),
                Err(_) => {}
            }
        }
    }

    fn handle_stop() {}
}

edgeless_function::export!(SystemTest);
