// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_function::*;

/// Function that delays any incoming UTF8 message by a given period, in ms,
/// specified in the init-payload annotation as delay_ms=PERIOD.
///
/// Only the cast() method is supported.
struct SloppyTest;

struct Conf {
    delay_ms: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct SelfMessage {
    secret: u64,
    msg: String,
}

static CONF: std::sync::OnceLock<Conf> = std::sync::OnceLock::new();

impl EdgeFunction for SloppyTest {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        if let Ok(encoded_message_str) = core::str::from_utf8(encoded_message) {
            if let Ok(self_message) = serde_json::from_str::<SelfMessage>(encoded_message_str) {
                if self_message.secret == 42 {
                    cast("out", self_message.msg.as_bytes());
                    return;
                }
            }
            let conf = CONF.get().unwrap();
            let self_message = SelfMessage {
                secret: 42,
                msg: encoded_message_str.to_string(),
            };
            delayed_cast(conf.delay_ms, "self", serde_json::to_string(&self_message).unwrap_or_default().as_bytes());
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        // edgeless_function::init_logger();

        let arguments = edgeless_function::init_payload_to_args(payload);
        let delay_ms = arguments.get("delay_ms").unwrap_or(&"1000").parse::<u64>().unwrap_or(1000);
        let _ = CONF.set(Conf { delay_ms });
    }

    fn handle_stop() {}
}

edgeless_function::export!(SloppyTest);
