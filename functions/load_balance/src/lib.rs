// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub use edgeless_function::*;

struct LoadBalanceFunction;

struct Conf {
    outputs: Vec<String>,
}

struct State {
    last_output: usize,
}

static CONF: std::sync::OnceLock<Conf> = std::sync::OnceLock::new();
static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

/// Function that serves in round-robin multiple outputs.
///
/// The expected output channels are called outX, where X goes from 1
/// to the number of outputs, which is specified in the init_payload
/// annotation. Example
///
/// "init_payload" : "num_outputs=3"
///
/// The function supports only asynchronous events, i.e., cast(), and it is
/// meant as an intermediate box to be put in between two logical
/// function/resource instances to perform load balancing.
///
/// For instance, the following workflow:
///
///     output
/// f ---------> g
///
/// Can be transformed into this one to improve throughput, if the function g()
/// is resource-bound can be parallelized in three instances:
///
///                                out1
///                            +---------> g1
///     output                 |   out2
/// f ---------> load_balance -+---------> g2
///                            |   out3
///                            +---------> g3
///
impl EdgeFunction for LoadBalanceFunction {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        let outputs = &CONF.get().unwrap().outputs;

        // Drop the incoming message if there are no configured outputs.
        if outputs.is_empty() {
            return;
        }

        let mut state = STATE.get().unwrap().lock().unwrap();

        state.last_output = (state.last_output + 1) % outputs.len();
        cast(&outputs[state.last_output], encoded_message);
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::Err
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        // Parse the initialization parameters.
        let arguments = edgeless_function::init_payload_to_args(payload);
        let num_outputs = arguments.get("num_outputs").unwrap_or(&"0").parse::<usize>().unwrap_or(0);

        // Save the configuration.
        let mut outputs = vec![];
        for i in 1..=num_outputs {
            outputs.push(format!("out{}", i));
        }
        let _ = CONF.set(Conf { outputs });

        // Initialized the internal state.
        let _ = STATE.set(std::sync::Mutex::new(State { last_output: 0 }));
    }

    fn handle_stop() {}
}

edgeless_function::export!(LoadBalanceFunction);
