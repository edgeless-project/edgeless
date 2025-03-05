// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use std::process::exit;

use dda;
use edgeless_function::*;

struct Conf {
    inter_arrival: u64,
}

struct State {
    next_id: usize,
}

static CONF: std::sync::OnceLock<Conf> = std::sync::OnceLock::new();
static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

struct DDAFunc;

impl EdgeFunction for DDAFunc {
    fn handle_cast(_source: InstanceId, encoded_message: &[u8]) {
        let conf = CONF.get().unwrap();
        let mut state = STATE.get().unwrap().lock().unwrap();

        delayed_cast(conf.inter_arrival, "self", b"");

        state.next_id += 1;
        let id = state.next_id - 1;

        // TODO: is that for sure okay to reuse the same id for workflow and function?
        cast("metric", format!("workflow:begin:{}", id).as_bytes());
        cast("metric", format!("function:begin:{}", id).as_bytes());

        // blocking call to the dda
        match dda::publish_action("actor", vec![]) {
            Ok(res) => log::debug!("action publish successful: {:?}", res),
            Err(e) => log::error!("action publish failed: {}", e),
        }

        cast("metric", format!("function:end:{}", id).as_bytes());
        cast("metric", format!("workflow:end:{}", id).as_bytes());
    }

    fn handle_call(_source: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    // example of payload:
    // inter_arrival=2000
    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        // TODO: parse the config options from the benchmark
        let arguments = if let Some(payload) = payload {
            let str_payload = core::str::from_utf8(payload).unwrap();
            edgeless_function::parse_init_payload(str_payload)
        } else {
            std::collections::HashMap::new()
        };
        let inter_arrival = arguments.get("inter_arrival").unwrap_or(&"1000").parse::<u64>().unwrap_or(1000);
        let _ = CONF.set(Conf { inter_arrival });
        let _ = STATE.set(std::sync::Mutex::new(State { next_id: 0 }));

        edgeless_function::init_logger();

        // initial cast
        delayed_cast(1000, "self", b"");
    }

    fn handle_stop() {}
}

edgeless_function::export!(DDAFunc);
