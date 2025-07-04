// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use dda;
use edgeless_function::*;

struct Conf {
    is_client: bool,
    is_sink: bool,
}

struct State {
    next_id: usize,
}

static CONF: std::sync::OnceLock<Conf> = std::sync::OnceLock::new();
static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

struct DDAChain;

impl EdgeFunction for DDAChain {
    fn handle_cast(_source: InstanceId, encoded_message: &[u8]) {
        let conf = CONF.get().unwrap();
        let mut state = STATE.get().unwrap().lock().unwrap();

        if conf.is_sink {
            // TODO: workflow has finished
            cast("metric", format!("workflow:end:{}", id).as_bytes());
            return;
        }

        if conf.is_client {
            // client just calls the next function in chain
            cast("out", &vec![]);
            // client self invokes every 100ms
            delayed_cast(100, "self", &vec![]);
        } else {
            // make a dda call
            match dda::publish_action("actor", vec![]) {
                Ok(res) => log::debug!("action publish successful: {:?}", res),
                Err(e) => log::error!("action publish failed: {}", e),
            }

            // cast to the next function in chain
            cast("out", &vec![]);
        }
    }

    fn handle_call(_source: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    // example of payload:
    // is_client=true
    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        let arguments = if let Some(payload) = payload {
            let str_payload = core::str::from_utf8(payload).unwrap();
            edgeless_function::parse_init_payload(str_payload)
        } else {
            std::collections::HashMap::new()
        };
        let is_client = arguments.get("is_client").unwrap_or(&"false").to_lowercase() == "true";
        let is_sink = arguments.get("is_sink").unwrap_or(&"false").to_lowercase() == "true";
        let _ = CONF.set(Conf { is_client, is_sink });
        let _ = STATE.set(std::sync::Mutex::new(State { next_id: 0 }));

        edgeless_function::init_logger();

        // initial cast
        if is_client {
            delayed_cast(1000, "self", b"");
        }
    }

    fn handle_stop() {}
}

edgeless_function::export!(DDAChain);
