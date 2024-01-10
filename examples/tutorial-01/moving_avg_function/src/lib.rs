// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::api::*;
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

impl Edgefunction for MovingAvgFunction {
    fn handle_cast(src: InstanceId, encoded_message: String) {
        log::info!(
            "moving_avg casted, node {}, function {}, MSG: {}",
            src.node,
            src.function,
            &encoded_message
        );

        let init_state = INIT_STATE.get().unwrap();

        match encoded_message.parse::<f32>() {
            Ok(val) => {
                let mut state = STATE.get().unwrap().lock().unwrap();
                if state.values.len() == init_state.num_values {
                    state.values.pop_back();
                }
                state.values.push_front(val);
                if state.values.len() == init_state.num_values {
                    let average: f32 = state.values.iter().sum();
                    cast(&"output", format!("{}", average / state.values.len() as f32).as_str());
                }
            }
            Err(err) => cast(&"error", format!("invalid event payload '{}': {}", &encoded_message, err).as_str()),
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: String) -> CallRet {
        CallRet::Noreply
    }

    fn handle_init(payload: String, _serialized_state: Option<String>) {
        edgeless_function::init_logger();

        let num_values = match payload.parse::<usize>() {
            Ok(val) => val,
            Err(_) => 0,
        };
        let _ = INIT_STATE.set(InitState { num_values });
        let _ = STATE.set(std::sync::Mutex::new(State { values: VecDeque::new() }));

        log::info!("moving_avg initialized with size = {}", num_values);
    }

    fn handle_stop() {
        log::info!("moving_avg stopped");
    }
}

edgeless_function::export!(MovingAvgFunction);
