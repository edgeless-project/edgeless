// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::api::*;
use log;

struct MessagingTest;

impl Edgefunction for MessagingTest {
    fn handle_cast(src: InstanceId, encoded_message: String) {
        match encoded_message.as_str() {
            "test_cast_raw_output" => {
                cast_raw(&src, "cast_raw_output");
            }
            "test_call_raw_output" => {
                let _res = call_raw(&src, "call_raw_output");
            }
            "test_delayed_cast_output" => {
                delayed_cast(100, "test", "delayed_cast_output");
            }
            "test_cast_output" => {
                cast("test", "cast_output");
            }
            "test_call_output" => {
                let _res = call("test", "call_output");
            }
            _ => {
                log::info!("Unprocessed Message");
            }
        }
    }

    fn handle_call(_src: InstanceId, encoded_message: String) -> CallRet {
        match encoded_message.as_str() {
            "test_err" => CallRet::Err,
            "test_ret" => CallRet::Reply("test_reply".to_string()),
            _ => CallRet::Noreply,
        }
    }

    fn handle_init(_payload: String, _serialized_state: Option<String>) {
        edgeless_function::init_logger();
        log::info!("Messaging Test Init");
    }

    fn handle_stop() {
        log::info!("Messaging Test Stop");
    }
}

edgeless_function::export!(MessagingTest);
