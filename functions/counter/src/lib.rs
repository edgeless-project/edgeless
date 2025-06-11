// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct Counter;

impl EdgeFunction for Counter {
    fn handle_cast(_src: InstanceId, message: &[u8]) {
        let msg_str = core::str::from_utf8(message).unwrap();
        let prev_count = msg_str.parse::<i32>().unwrap();
        let cur_count = format!("{}", prev_count + 1);
        cast("output", cur_count.as_bytes());
        delayed_cast(1000, "self", cur_count.as_bytes());
    }

    fn handle_call(_src: InstanceId, _message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(init_message: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        let message = if let Some(init_message) = init_message {
            let init_msg_str = core::str::from_utf8(init_message).unwrap();
            if init_msg_str.parse::<i32>().is_ok() {
                init_message
            } else {
                b"0"
            }
        } else {
            b"0"
        };
        cast("self", message);
    }

    fn handle_stop() {
        // noop
    }
}

edgeless_function::export!(Counter);
