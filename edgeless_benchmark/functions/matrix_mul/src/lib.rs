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
    inter_arrival: std::time::Duration,
    is_first: bool,
    is_last: bool,
    wf_name: String,
    fun_name: String,
    outputs: std::collections::HashSet<String>,
}
struct State {
    next_id: usize,
    next_arrival: std::time::Instant,
    lcg: Lcg,
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
        let ts_start = std::time::Instant::now();

        let conf = CONF.get().unwrap();
        log::info!("MatrixMul casted, wf {}, fun {}, MSG: {}", conf.wf_name, conf.fun_name, encoded_message);

        let mut state = STATE.get().unwrap().lock().unwrap();
        let n = state.matrix.len();

        // Fill a new matrix with random numbers.
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
        let ts_end = std::time::Instant::now();
        let id = match conf.is_first {
            true => state.next_id,
            false => encoded_message.parse::<usize>().unwrap_or(0),
        };

        // Save metrics.
        if conf.is_first {
            cast(
                "metrics",
                format!("{}:{}:start:{}", conf.wf_name, id, ts_start.elapsed().as_micros()).as_str(),
            );
        }
        if conf.is_last {
            cast(
                "metrics",
                format!("{}:{}:end:{}", conf.wf_name, id, ts_end.elapsed().as_micros()).as_str(),
            );
        }
        cast(
            "metrics",
            format!("{}:{}:{}:{}", conf.wf_name, conf.fun_name, id, (ts_end - ts_start).as_micros()).as_str(),
        );

        // Call outputs
        for output in &conf.outputs {
            cast(output, format!("{}", id).as_str());
        }

        // Schedule next event.
        if conf.is_first {
            state.next_id += 1;
            if state.next_arrival > ts_end {
                let time_until_next_arrival = state.next_arrival - ts_end;
                delayed_cast(time_until_next_arrival.as_millis() as u64, "self", "");
            } else {
                cast("self", "");
            }
            state.next_arrival += conf.inter_arrival;
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: String) -> CallRet {
        log::info!("MatrixMul called: ignored");
        CallRet::Noreply
    }

    // example of payload:
    // seed=42,inter_arrival=5.0,is_first=true,is_last=false,wf_name=my_workflow,fun_name=my_function,matrix_size=1000,outputs=0:2:19
    fn handle_init(payload: String, _serialized_state: Option<String>) {
        edgeless_function::init_logger();
        log::info!("MatrixMul initialized, payload: {}", payload);
        let arguments = parse_init(&payload);

        let seed = arguments.get("seed").unwrap_or(&"0").parse::<u32>().unwrap_or(0);

        let inter_arrival = std::time::Duration::from_secs_f32(arguments.get("inter_arrival").unwrap_or(&"1.0").parse::<f32>().unwrap_or(1.0));
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
            outputs,
        });

        let mut lcg = Lcg::new(seed);
        let matrix = make_new_matrix(&mut lcg, matrix_size);

        log::info!("XXX");
        let t = std::time::Instant::now();
        log::info!("XXX {:?}", t);
        let next_arrival = t + inter_arrival;
        log::info!("XXX {:?}", t);

        let _ = STATE.set(std::sync::Mutex::new(State {
            next_id: 0,
            next_arrival,
            lcg,
            matrix,
        }));

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
}
