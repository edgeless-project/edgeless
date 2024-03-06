// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_function::*;

struct MatrixMulFunction;

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
    lcg: edgeless_function::lcg::Lcg,
    // Matrix of values to consume CPU.
    matrix: Vec<f32>,
}

static CONF: std::sync::OnceLock<Conf> = std::sync::OnceLock::new();
static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

impl EdgeFunction for MatrixMulFunction {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        let conf = CONF.get().unwrap();
        // log::info!("MatrixMul casted, wf {}, fun {}, MSG: {:?}", conf.wf_name, conf.fun_name, encoded_message);
        let mut state = STATE.get().unwrap().lock().unwrap();

        // Schedule the next transaction.
        let id = match conf.is_first {
            true => {
                if conf.inter_arrival > 0 {
                    delayed_cast(conf.inter_arrival, "self", b"");
                }
                cast("metric", format!("workflow:start:{}:{}", conf.wf_name, state.next_id).as_bytes());
                state.next_id += 1;
                state.next_id - 1
            }
            false => core::str::from_utf8(encoded_message).unwrap_or("0").parse::<usize>().unwrap_or(0),
        };
        cast("metric", format!("function:start:{}:{}:{}", conf.wf_name, conf.fun_name, id).as_bytes());

        // Fill a new matrix with random numbers.
        let n = conf.matrix_size;
        let random_matrix = edgeless_function::lcg::random_matrix(&mut state.lcg, n);

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
            cast("metric", format!("workflow:end:{}:{}", conf.wf_name, id).as_bytes());
        }
        cast("metric", format!("function:end:{}:{}:{}", conf.wf_name, conf.fun_name, id).as_bytes());

        // Call outputs
        for output in &conf.outputs {
            cast(output, format!("{}", id).as_bytes());
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        log::info!("MatrixMul called: ignored");
        CallRet::NoReply
    }

    // example of payload:
    // seed=42,inter_arrival=2000,is_first=true,is_last=false,wf_name=my_workflow,fun_name=my_function,matrix_size=1000,outputs=0:2:19
    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("MatrixMul initialized, payload: {:?}", payload);
        

        let arguments = if let Some(payload) = payload {
            let str_payload = core::str::from_utf8(payload).unwrap();
            edgeless_function::parse_init_payload(str_payload)
        } else {
            std::collections::HashMap::new()
        };

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

        let mut lcg = edgeless_function::lcg::Lcg::new(seed);
        let matrix = edgeless_function::lcg::random_matrix(&mut lcg, matrix_size);

        let _ = STATE.set(std::sync::Mutex::new(State { next_id: 0, lcg, matrix }));

        if is_first {
            delayed_cast(1000, "self", b"");
        }
    }

    fn handle_stop() {
        let conf = CONF.get().unwrap();
        log::info!("MatrixMul stopped, wf {}, fun {}", conf.wf_name, conf.fun_name);
    }
}

edgeless_function::export!(MatrixMulFunction);
