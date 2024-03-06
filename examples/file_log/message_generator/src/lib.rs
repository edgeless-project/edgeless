// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct MessageGenerator;

impl EdgeFunction for MessageGenerator {
    fn handle_cast(src: InstanceId, message: &[u8]) {
        cast(
            "output",
            format!(
                "{} from {:?}:{:?}",
                &core::str::from_utf8(message).unwrap(),
                uuid::Uuid::from_bytes(src.node_id).to_string(),
                uuid::Uuid::from_bytes(src.component_id).to_string()
            )
            .as_bytes(),
        );
        delayed_cast(1000, "self", &message);
    }

    fn handle_call(_src: InstanceId, _message: &[u8]) -> CallRet {
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

edgeless_function::export!(MessageGenerator);
