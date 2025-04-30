// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct Counter;

impl EdgeFunction for Counter {
    fn handle_cast(_src: InstanceId, message: &[u8]) {
        let msg_str = core::str::from_utf8(message).unwrap();
        let prev_count = msg_str.parse::<i32>().unwrap();
        let cur_count = format!("{}", prev_count + 1);
        cast("redis", cur_count.as_bytes());
        cast("output", cur_count.as_bytes());
        delayed_cast(1000, "self", cur_count.as_bytes());
    }

    fn handle_call(_src: InstanceId, _message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(init_message: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        let message = if let Some(init_message) = init_message {
            match call("redis", "last_counter".as_bytes()) {
                CallRet::Reply(owned_byte_buff) => {
                    if let Ok(res) = core::str::from_utf8(&owned_byte_buff) {
                        match res.parse::<i32>() {
                            Ok(_) => res.to_string(),
                            Err(_) => String::from("0"),
                        }
                    } else {
                        String::from("0")
                    }
                }
                CallRet::Err | CallRet::NoReply => {
                    let init_msg_str = core::str::from_utf8(init_message).unwrap();
                    match init_msg_str.parse::<i32>() {
                        Ok(_) => init_msg_str.to_string(),
                        Err(_) => String::from("0"),
                    }
                }
            }
        } else {
            String::from("0")
        };
        cast("self", &message.as_bytes());
    }

    fn handle_stop() {
        // noop
    }
}

edgeless_function::export!(Counter);
