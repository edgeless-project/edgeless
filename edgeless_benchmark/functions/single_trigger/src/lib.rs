// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_function::*;

/// Function that simply casts a message with a payload given in the init().
struct SingleTriggerFunction;

impl EdgeFunction for SingleTriggerFunction {
    fn handle_cast(_src: InstanceId, _encoded_message: &[u8]) {
        // Noop
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        // edgeless_function::init_logger();
        if let Some(message) = payload {
            cast("out", message);
        }
    }

    fn handle_stop() {
        // Noop
    }
}

edgeless_function::export!(SingleTriggerFunction);
