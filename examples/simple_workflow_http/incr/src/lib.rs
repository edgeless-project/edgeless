// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct IncrFun;

impl EdgeFunction for IncrFun {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        let str_message = core::str::from_utf8(encoded_message).unwrap();

        log::info!("incr: called with '{}'", str_message);

        if let Ok(n) = str_message.parse::<i32>() {
            cast("result", format!("{}", n + 1).as_bytes());
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, _init_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("incr: started");
    }

    fn handle_stop() {
        log::info!("incr: stopped");
    }
}

edgeless_function::export!(IncrFun);
