// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use std::collections::HashSet;
use edgeless_function::*;

struct Conf {
    num_middle_blocks: usize,
}

struct State {
    received_partial_results: HashSet<usize>,
}

// conf can only be written once
static CONF: std::sync::OnceLock<Conf> = std::sync::OnceLock::new();
static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

struct DDAChainLast;

// ..........workflow starts
//     ┌─────────┐
//     │  First  │
//     │  func.  │
//     └────┬────┘
//          │
//          ▼
//     ┌────────────┐
//     │ Fan-out to │
//     │chain funcs │
//     │ 1..n blocks│
//     └─┬─┬─┬─┬─┬─-┘
//       │ │ │ │ │
//       ▼ ▼ ▼ ▼ ▼
//     ┌─┐┌─┐┌─┐┌─┐┌─┐
//     │1││2││3││4││n│
//     │ ││ ││ ││ ││ │
//     └─┘└─┘└─┘└─┘└─┘
//       │ │ │ │ │
//       └─┼─┼─┼─┘
//         └─┼─┘
//           ▼
//     ┌─────────┐
//     │ Fan-in  │
//     │  Block  │
//     └─────────┘
// ..........workflow ends
impl EdgeFunction for DDAChainLast {
    fn handle_cast(_source: InstanceId, encoded_message: &[u8]) {
        let conf = CONF.get().expect("Configuration should be initialized");
        let mut state = STATE.get().expect("State should be initialized").lock().expect("Failed to acquire state lock");

        // last function - check if we have all partial results
        let identifier: &str = core::str::from_utf8(encoded_message).expect("Message should be valid UTF-8");
        // parse from {workflow_id}-{function_id}
        let parts: Vec<&str> = identifier.split('-').collect();
        if parts.len() != 2 {
            log::warn!("invalid partial result identifier: {}", identifier);
            return;
        }
        let workflow_id: &str = parts[0];
        let function_id: &str = parts[1];

        if state.received_partial_results.contains(&function_id.parse::<usize>().expect("Function ID should be a valid number")) {
            log::warn!("duplicate partial result received: {}", identifier);
            return;
        }
        state.received_partial_results.insert(function_id.parse::<usize>().expect("Function ID should be a valid number"));
        if state.received_partial_results.len() == conf.num_middle_blocks {
            // workflow has finished
            telemetry_log(5, "workflow:end", workflow_id);

            // clear received partial results
            state.received_partial_results.clear();

            // cast to the first function in workflow which will fan-out again
            cast("first", &vec![]); // empty message
        }
    }

    fn handle_call(_source: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    // example of payload:
    // is_last=true,num_middle_blocks=3
    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        let arguments = if let Some(payload) = payload {
            let str_payload = core::str::from_utf8(payload).expect("Init payload should be valid UTF-8");
            edgeless_function::parse_init_payload(str_payload)
        } else {
            std::collections::HashMap::new()
        };
        let num_middle_blocks = arguments.get("num_middle_blocks").expect("num_middle_blocks should be set");
        let num_middle_blocks = num_middle_blocks.parse::<usize>().expect("num_middle_blocks should be a valid number");
        let _ = CONF.set(Conf { num_middle_blocks });
        let _ = STATE.set(std::sync::Mutex::new(State { received_partial_results: HashSet::new() }));

        edgeless_function::init_logger();
    }

    fn handle_stop() {}
}

edgeless_function::export!(DDAChainLast);
