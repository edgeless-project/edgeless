// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::api::*;

struct MessageGenerator;

impl Edgefunction for MessageGenerator {
    fn handle_cast(src: InstanceId, message: String) {
        cast("output", format!("{} from {}:{}", &message, src.node, src.function).as_str());
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

edgeless_function::export!(MessageGenerator);
