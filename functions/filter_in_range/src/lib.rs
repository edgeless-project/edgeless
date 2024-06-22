// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct FilterInRangeFunction;

struct InitState {
    min_value: f32,
    max_value: f32,
}

static INIT_STATE: std::sync::OnceLock<InitState> = std::sync::OnceLock::new();

impl EdgeFunction for FilterInRangeFunction {
    fn handle_cast(src: InstanceId, encoded_message: &[u8]) {
        let str_message = core::str::from_utf8(encoded_message).unwrap();

        log::info!(
            "Filter_in_range casted, node {:?}, function {:?}, MSG: {}",
            src.node_id,
            src.component_id,
            &str_message
        );

        match str_message.parse::<f32>() {
            Ok(val) => {
                let state = INIT_STATE.get().unwrap();
                if val >= state.min_value && val <= state.max_value {
                    cast(&"output", encoded_message);
                } else {
                    cast(
                        &"error",
                        format!("value '{}' out of range [{},{}]", val, state.min_value, state.max_value).as_bytes(),
                    );
                }
            }
            Err(err) => cast(&"error", format!("invalid event payload '{}': {}", &str_message, err).as_bytes()),
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();

        if let Some(payload) = payload {
            let payload_str = core::str::from_utf8(payload).unwrap();
            let tokens: Vec<&str> = payload_str.split(',').collect();
            let mut min_value = 0.0;
            let mut max_value = 0.0;
            if tokens.len() == 2 {
                if let (Ok(lhs), Ok(rhs)) = (tokens[0].parse::<f32>(), tokens[1].parse::<f32>()) {
                    min_value = lhs;
                    max_value = rhs;
                }
            }
            log::info!("Filter_in_range initialized with [{},{}]", min_value, max_value);
            let _ = INIT_STATE.set(InitState { min_value, max_value });
        }
    }

    fn handle_stop() {
        log::info!("Filter_in_range stopped");
    }
}

edgeless_function::export!(FilterInRangeFunction);
