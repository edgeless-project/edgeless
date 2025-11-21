// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use edgeless_function::*;

struct DDAChainFirst;

struct State {
    invocation_sequence_number: usize,
}

struct Conf {
    chain_length: usize,
}

// conf can only be written to once
static CONF: std::sync::OnceLock<Conf> = std::sync::OnceLock::new();
static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

impl EdgeFunction for DDAChainFirst {
    fn handle_cast(_source: InstanceId, _encoded_message: &[u8]) {
        let mut state = STATE.get().unwrap().lock().unwrap();
        if _encoded_message != b"first" {
            telemetry_log(5, "workflow:end", &state.invocation_sequence_number.to_string());
            // for the next workflow
            state.invocation_sequence_number += 1;
        }
        telemetry_log(5, "workflow:start", &state.invocation_sequence_number.to_string());
        // invoke all the functions at the same time through casting to them
        let conf = CONF.get().unwrap();
        for i in 0..conf.chain_length {
            let target = format!("func{}", i);
            let encoded_message = format!("{}-{}", state.invocation_sequence_number, i);
            // payload is a tuple of (invocation_sequence_number, function_index)
            cast(&target, &encoded_message.into_bytes());
        }
    }

    fn handle_call(_source: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        let arguments = if let Some(payload) = payload {
            let str_payload = core::str::from_utf8(payload).unwrap();
            edgeless_function::parse_init_payload(str_payload)
        } else {
            std::collections::HashMap::new()
        };
        let chain_length: usize = arguments.get("chain_length").expect("chain_length should be set").parse().expect("chain_length should be a valid number");
        let _ = CONF.set(Conf { chain_length });
        let _ = STATE.set(std::sync::Mutex::new(State { invocation_sequence_number: 0 }));

        // Start the sequence / chain / dda experiment
        delayed_cast(50, "self", b"first");
    }

    fn handle_stop() {}
}

edgeless_function::export!(DDAChainFirst);
