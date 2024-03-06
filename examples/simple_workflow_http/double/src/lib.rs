// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct DoubleFun;

impl EdgeFunction for DoubleFun {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        let str_message = core::str::from_utf8(encoded_message).unwrap();

        log::info!("double: called with '{}'", str_message);

        if let Ok(n) = str_message.parse::<i32>() {
            cast("result", format!("{}", 2 * n).as_bytes());
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, _init_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("double: started");
    }

    fn handle_stop() {
        log::info!("double: stopped");
    }
}

edgeless_function::export!(DoubleFun);
