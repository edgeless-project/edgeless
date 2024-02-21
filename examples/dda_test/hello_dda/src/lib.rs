// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use edgeless_function::api::*;

struct HelloDDA;

impl Edgefunction for HelloDDA {
    fn handle_cast(src: InstanceId, message: String) {
        cast("file_log_output", "HelloDDA");
        cast("dda_output", "Hello DDA");

        delayed_cast(1000, "self", &message);
    }

    fn handle_call(_src: InstanceId, _message: String) -> CallRet {
        CallRet::Noreply
    }

    fn handle_init(init_message: String, _serialized_state: Option<String>) {
        edgeless_function::init_logger();
        cast("self", &init_message);
    }

    fn handle_stop() {
        // noop
    }
}

edgeless_function::export!(HelloDDA);
