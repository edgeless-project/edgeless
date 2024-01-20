// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_function::api::*;
use std::num::Wrapping;

struct MatrixMulFunction;

// Parameters from glib's implementation.
const MODULUS: Wrapping<u32> = Wrapping(2147483648);
const MULTIPLIER: Wrapping<u32> = Wrapping(1103515245);
const OFFSET: Wrapping<u32> = Wrapping(12345);

struct Lcg {
    seed: Wrapping<u32>,
}

impl Lcg {
    fn new(seed: u32) -> Self {
        Self { seed: Wrapping(seed) }
    }

    fn rand(&mut self) -> f32 {
        self.seed = (MULTIPLIER * self.seed + OFFSET) % MODULUS;
        self.seed.0 as f32 / MODULUS.0 as f32
    }
}

fn make_new_matrix(lcg: &mut Lcg, size: usize) -> Vec<f32> {
    let mut new_matrix = vec![0 as f32; size * size];
    for value in new_matrix.iter_mut() {
        *value = lcg.rand();
    }
    new_matrix
}

struct Conf {
    // Interarrival time before the next event, in ms. Only if is_first == true.
    inter_arrival: u64,
    // True: this is the first function in the workflow.
    is_first: bool,
    // True: this is the last function in the workflow.
    is_last: bool,
    // Name of the workflow (for stats only).
    wf_name: String,
    // Name of the function (for stats only).
    fun_name: String,
    // Matrix size.
    matrix_size: usize,
    // Which outputs are enabled.
    outputs: std::collections::HashSet<String>,
}
struct State {
    // ID of the next transaction. Only used if is_first == true.
    next_id: usize,
    // Pseudo-random number generator.
    lcg: Lcg,
    // Matrix of values to consume CPU.
    matrix: Vec<f32>,
}

static CONF: std::sync::OnceLock<Conf> = std::sync::OnceLock::new();
static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

fn parse_init(payload: &str) -> std::collections::HashMap<&str, &str> {
    let tokens = payload.split(',');
    let mut arguments = std::collections::HashMap::new();
    for token in tokens {
        let mut inner_tokens = token.split('=');
        if let Some(key) = inner_tokens.next() {
            if let Some(value) = inner_tokens.next() {
                arguments.insert(key, value);
            } else {
                log::error!("invalid initialization token: {}", token);
            }
        } else {
            log::error!("invalid initialization token: {}", token);
        }
    }
    arguments
}

impl Edgefunction for MatrixMulFunction {
    fn handle_cast(_src: InstanceId, encoded_message: String) {
        let conf = CONF.get().unwrap();
        log::info!("MatrixMul casted, wf {}, fun {}, MSG: {}", conf.wf_name, conf.fun_name, encoded_message);
        let mut state = STATE.get().unwrap().lock().unwrap();

        // Schedule the next transaction.
        let id = match conf.is_first {
            true => {
                delayed_cast(conf.inter_arrival, "self", "");
                cast("metric", format!("workflow:start:{}:{}", conf.wf_name, state.next_id).as_str());
                state.next_id += 1;
                state.next_id - 1
            }
            false => encoded_message.parse::<usize>().unwrap_or(0),
        };
        cast("metric", format!("function:start:{}:{}:{}", conf.wf_name, conf.fun_name, id).as_str());

        // Fill a new matrix with random numbers.
        let n = conf.matrix_size;
        let random_matrix = make_new_matrix(&mut state.lcg, n);

        // Multiply previous matrix by the random one.
        let mut output_matrix = vec![0 as f32; n * n];
        for i in 0..n {
            // output row
            for j in 0..n {
                // output col
                let mut sum = 0.0_f32;
                for k in 0..n {
                    sum += state.matrix[i * n + k] * random_matrix[k * n + j];
                }
                output_matrix[i * n + j] = sum;
            }
        }
        state.matrix = output_matrix;

        // Save metrics at the end of the execution.
        if conf.is_last {
            cast("metric", format!("workflow:end:{}:{}", conf.wf_name, id).as_str());
        }
        cast("metric", format!("function:end:{}:{}:{}", conf.wf_name, conf.fun_name, id).as_str());

        // Call outputs
        for output in &conf.outputs {
            cast(output, format!("{}", id).as_str());
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: String) -> CallRet {
        log::info!("MatrixMul called: ignored");
        CallRet::Noreply
    }

    // example of payload:
    // seed=42,inter_arrival=2000,is_first=true,is_last=false,wf_name=my_workflow,fun_name=my_function,matrix_size=1000,outputs=0:2:19
    fn handle_init(payload: String, _serialized_state: Option<String>) {
        edgeless_function::init_logger();
        log::info!("MatrixMul initialized, payload: {}", payload);
        let arguments = parse_init(&payload);

        let seed = arguments.get("seed").unwrap_or(&"0").parse::<u32>().unwrap_or(0);

        let inter_arrival = arguments.get("inter_arrival").unwrap_or(&"1000").parse::<u64>().unwrap_or(1000);
        let is_first = arguments.get("is_first").unwrap_or(&"false").to_lowercase() == "true";
        let is_last = arguments.get("is_last").unwrap_or(&"false").to_lowercase() == "true";
        let wf_name = arguments.get("wf_name").unwrap_or(&"no-wf-name").to_string();
        if wf_name == "no-wf-name" {
            log::warn!("workflow name not specified, using: no-wf-name");
        }
        let fun_name = arguments.get("fun_name").unwrap_or(&"no-fun-name").to_string();
        if fun_name == "no-fun-name" {
            log::warn!("workflow name not specified, using: no-fun-name");
        }
        let matrix_size = arguments.get("matrix_size").unwrap_or(&"100").parse::<usize>().unwrap_or(100);
        let output_value = arguments.get("outputs").unwrap_or(&"0").to_string();
        let output_tokens = output_value.split(':');
        let mut outputs = std::collections::HashSet::new();
        for n in output_tokens.into_iter().map(|x| x.parse::<usize>().unwrap_or(0)).collect::<Vec<usize>>() {
            outputs.insert(format!("out-{}", n));
        }

        let _ = CONF.set(Conf {
            inter_arrival,
            is_first,
            is_last,
            wf_name,
            fun_name,
            matrix_size,
            outputs,
        });

        let mut lcg = Lcg::new(seed);
        let matrix = make_new_matrix(&mut lcg, matrix_size);

        let _ = STATE.set(std::sync::Mutex::new(State { next_id: 0, lcg, matrix }));

        if is_first {
            cast("self", "");
        }
    }

    fn handle_stop() {
        let conf = CONF.get().unwrap();
        log::info!("MatrixMul stopped, wf {}, fun {}", conf.wf_name, conf.fun_name);
    }
}

edgeless_function::export!(MatrixMulFunction);

#[cfg(test)]
mod test {
    use crate::make_new_matrix;
    use crate::parse_init;
    use crate::Lcg;

    #[test]
    fn test_matrix_mul_parse_init() {
        assert_eq!(
            std::collections::HashMap::from([("a", "b"), ("c", "d"), ("my_key", "my_value")]),
            parse_init("a=b,c=d,my_key=my_value")
        );

        assert_eq!(
            std::collections::HashMap::from([("a", ""), ("", "d"), ("my_key", "my_value")]),
            parse_init("a=,=d,my_key=my_value")
        );

        assert_eq!(
            std::collections::HashMap::from([("my_key", "my_value")]),
            parse_init("a,d,my_key=my_value")
        );

        assert!(parse_init(",,,a,s,s,,42,").is_empty());
    }

    #[test]
    fn test_matrix_mul_lcg() {
        let mut numbers = std::collections::HashSet::new();
        let mut lcg = Lcg::new(42);
        for _ in 0..1000 {
            let rnd = lcg.rand();
            numbers.insert((rnd * 20.0).floor() as u32);
        }
        assert_eq!(20, numbers.len());
    }

    #[test]
    fn test_matrix_mul_make_new_matrix() {
        let mut lcg = Lcg::new(42);
        let matrix = make_new_matrix(&mut lcg, 1000);
        assert_eq!(1000 * 1000, matrix.len());
        assert_ne!(0.0 as f32, matrix.iter().sum());
    }
}
