// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;
use log;
use std::collections::VecDeque;

struct MovingAvgFunction;

struct InitState {
    num_values: usize,
}

struct State {
    values: VecDeque<f32>,
}

static INIT_STATE: std::sync::OnceLock<InitState> = std::sync::OnceLock::new();
static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

impl EdgeFunction for MovingAvgFunction {
    fn handle_cast(src: InstanceId, encoded_message: &[u8]) {
        let str_message = core::str::from_utf8(encoded_message).unwrap();

        log::info!(
            "moving_avg casted, node {:?}, function {:?}, MSG: {}",
            src.node_id,
            src.component_id,
            &str_message
        );

        let init_state = INIT_STATE.get().unwrap();

        match str_message.parse::<f32>() {
            Ok(val) => {
                let mut state = STATE.get().unwrap().lock().unwrap();
                if state.values.len() == init_state.num_values {
                    state.values.pop_back();
                }
                state.values.push_front(val);
                if state.values.len() == init_state.num_values {
                    let average: f32 = state.values.iter().sum();
                    cast(&"output", format!("{}", average / state.values.len() as f32).as_bytes());
                }
            }
            Err(err) => cast(&"error", format!("invalid event payload '{:?}': {}", &encoded_message, err).as_bytes()),
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();

        if let Some(payload) = payload {
            let str_payload = core::str::from_utf8(payload).unwrap();
            let num_values = match str_payload.parse::<usize>() {
                Ok(val) => val,
                Err(_) => 0,
            };
            let _ = INIT_STATE.set(InitState { num_values });
            let _ = STATE.set(std::sync::Mutex::new(State { values: VecDeque::new() }));

            log::info!("moving_avg initialized with size = {}", num_values);
        }
    }

    fn handle_stop() {
        log::info!("moving_avg stopped");
    }
}

edgeless_function::export!(MovingAvgFunction);
