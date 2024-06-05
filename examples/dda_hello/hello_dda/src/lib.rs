// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use edgeless_function::*;

struct HelloDDA;

impl EdgeFunction for HelloDDA {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        cast("file_log_output", b"HelloDDA");
        cast("dda_output", b"Hello DDA");

        delayed_cast(1000, "self", &encoded_message);
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(init_message: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        if let Some(init_message) = init_message {
            cast("self", &init_message);
        }
    }

    fn handle_stop() {
        // noop
    }
}

edgeless_function::export!(HelloDDA);
