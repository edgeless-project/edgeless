// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;
use log;

struct MessagingTest;

impl EdgeFunction for MessagingTest {
    fn handle_cast(src: InstanceId, encoded_message: &[u8]) {
        match core::str::from_utf8(encoded_message).unwrap() {
            "test_cast_raw_output" => {
                cast_raw(src, "cast_raw_output".as_bytes());
            }
            "test_call_raw_output" => {
                let _res = call_raw(src, "call_raw_output".as_bytes());
            }
            "test_delayed_cast_output" => {
                delayed_cast(100, "test", "delayed_cast_output".as_bytes());
            }
            "test_cast_output" => {
                cast("test", "cast_output".as_bytes());
            }
            "test_call_output" => {
                let _res = call("test", "call_output".as_bytes());
            }
            _ => {
                log::info!("Unprocessed Message");
            }
        }
    }

    fn handle_call(_src: InstanceId, encoded_message: &[u8]) -> CallRet {
        match core::str::from_utf8(encoded_message).unwrap() {
            "test_err" => CallRet::Err,
            "test_ret" => CallRet::Reply(edgeless_function::OwnedByteBuff::new_from_slice("test_reply".as_bytes())),
            _ => CallRet::NoReply,
        }
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Messaging Test Init");
    }

    fn handle_stop() {
        log::info!("Messaging Test Stop");
    }
}

edgeless_function::export!(MessagingTest);
