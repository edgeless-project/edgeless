// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_function::*;

struct Conf {
    message: Vec<u8>,
}

static CONF: std::sync::OnceLock<Conf> = std::sync::OnceLock::new();

/// Function that simply casts a message with a payload given in the init().
struct SingleTriggerFunction;

impl EdgeFunction for SingleTriggerFunction {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        if encoded_message.is_empty() {
            cast("out", &CONF.get().unwrap().message);
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        // edgeless_function::init_logger();

        // Parse the initialization parameters.
        let arguments = edgeless_function::init_payload_to_args(payload);
        let delay_ms = arguments.get("delay_ms").unwrap_or(&"0").parse::<u64>().unwrap_or(0);
        let message = arguments.get("message").unwrap_or(&"").as_bytes().to_vec();

        delayed_cast(delay_ms, "self", &vec![]);

        let _ = CONF.set(Conf { message });
    }

    fn handle_stop() {
        // Noop
    }
}

edgeless_function::export!(SingleTriggerFunction);
