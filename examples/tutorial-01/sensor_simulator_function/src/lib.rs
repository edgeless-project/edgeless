// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;
use log;

struct SensorSimulatorFunction;

struct InitState {
    period: u64,
    min_value: f32,
    max_value: f32,
}

struct State {
    lcg: edgeless_function::lcg::Lcg,
}

static INIT_STATE: std::sync::OnceLock<InitState> = std::sync::OnceLock::new();
static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

impl EdgeFunction for SensorSimulatorFunction {
    fn handle_cast(src: InstanceId, _encoded_message: &[u8]) {
        let init_state = INIT_STATE.get().unwrap();
        let mut state = STATE.get().unwrap().lock().unwrap();
        let value = state.lcg.rand() * (init_state.max_value - init_state.min_value) + init_state.min_value;
        log::info!(
            "sensor_simulator {:?}:{:?}, new value generated: {}",
            src.node_id,
            src.component_id,
            value
        );
        cast(&"output", format!("{}", value).as_bytes());

        delayed_cast(init_state.period, "self", b"");
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
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

        let period = arguments.get("period").unwrap_or(&"1000").parse::<u64>().unwrap_or(1000);
        let min_value = arguments.get("min-value").unwrap_or(&"0.0").parse::<f32>().unwrap_or(0.0);
        let max_value = arguments.get("max-value").unwrap_or(&"1.0").parse::<f32>().unwrap_or(1.0);
        let _ = INIT_STATE.set(InitState {
            period,
            min_value,
            max_value,
        });

        let seed = arguments.get("seed").unwrap_or(&"0").parse::<u32>().unwrap_or(0);
        let lcg = edgeless_function::lcg::Lcg::new(seed);
        let _ = STATE.set(std::sync::Mutex::new(State { lcg }));

        cast("self", b"");
    }

    fn handle_stop() {
        // noop
    }
}

edgeless_function::export!(SensorSimulatorFunction);
