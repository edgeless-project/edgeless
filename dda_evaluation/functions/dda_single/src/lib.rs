// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use dda;
use edgeless_function::*;

struct Conf {
    inter_arrival: u64,
    random_number: u64,
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


        state.next_id += 1;
        let id = state.next_id - 1;
        
        // log::info!("DDAFunc received cast message: {:?}, id: {}", encoded_message, id);
        let correlation_id = CONF.get().unwrap().random_number;
        // cast("metric", format!("function:begin:{}", correlation_id).as_bytes());
        telemetry_log(2, "function:start", format!("{}-{}", correlation_id, id).as_str());

        // blocking call to the dda
        match dda::publish_action("actor", vec![]) {
            Ok(res) => {
                log::info!("action publish successful");
                telemetry_log(2, "function:end", format!("{}-{}", correlation_id, id).as_str());
            },
            Err(e) => log::info!("action publish failed"),
        }

        delayed_cast(conf.inter_arrival, "self", b"");
    }

    fn handle_call(_source: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    // example of payload:
    // inter_arrival=2000,random_number=12345
    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        let arguments = if let Some(payload) = payload {
            let str_payload = core::str::from_utf8(payload).unwrap();
            edgeless_function::parse_init_payload(str_payload)
        } else {
            std::collections::HashMap::new()
        };
        let inter_arrival = arguments.get("inter_arrival").unwrap_or(&"1000").parse::<u64>().unwrap_or(1000);
        let random_number = arguments.get("random_number").unwrap_or(&"42").parse::<u64>().unwrap_or(42);
        let _ = CONF.set(Conf { inter_arrival, random_number });
        let _ = STATE.set(std::sync::Mutex::new(State { next_id: 0 }));

        edgeless_function::init_logger();

        // initial cast
        delayed_cast(1000, "self", b"");
    }

    fn handle_stop() {}
}

edgeless_function::export!(DDAFunc);
