// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_function::*;

struct VectorMulFunction;

struct Conf {
    // True: this is the client, which triggers the first input and receives the last output.
    is_client: bool,
    // Input size of the vector.
    input_size: usize,
}
struct State {
    // ID of the next transaction. Only used if is_client == true.
    next_id: usize,
    // Pseudo-random number generator.
    lcg: edgeless_function::lcg::Lcg,
    // Matrix of values to consume CPU in processing functions. Unused by clients.
    matrix: Vec<f32>,
}

static CONF: std::sync::OnceLock<Conf> = std::sync::OnceLock::new();
static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

impl EdgeFunction for VectorMulFunction {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        let conf = CONF.get().unwrap();
        let mut state = STATE.get().unwrap().lock().unwrap();

        //
        // Client
        //
        if conf.is_client {
            let id = state.next_id;
            if id > 0 {
                telemetry_log(5, "tend", &id.to_string());
            }

            state.next_id += 1;
            let random_input = edgeless_function::lcg::random_vector(&mut state.lcg, conf.input_size);
            let payload = format!(
                "{},{}",
                state.next_id,
                random_input.iter().map(|x| format!("{}", x)).collect::<Vec<String>>().join(",")
            );

            telemetry_log(5, "tbegin", &state.next_id.to_string());
            cast("out", payload.as_bytes());

        //
        // Processing function
        //
        } else {
            let input = core::str::from_utf8(encoded_message)
                .unwrap_or("0.0")
                .split(',')
                .map(|x| x.parse::<f32>().unwrap_or(0.0))
                .collect::<Vec<f32>>();
            let n = conf.input_size;
            assert!(input.len() == (1 + n));
            let id = input[0] as usize;

            // Produce the output by multiplying the internal matrix by the input.
            let mut output = vec![0.0_f32; n];
            for i in 0..n {
                for j in 0..n {
                    output[i] += state.matrix[i * n + j] * input[j];
                }
            }
            cast(
                "out",
                format!("{},{}", id, output.iter().map(|x| format!("{}", x)).collect::<Vec<String>>().join(",")).as_bytes(),
            );
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    // example of payload:
    // seed=42,is_client=true,is_last=false,input_size=1000
    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        // edgeless_function::init_logger();
        let arguments = if let Some(payload) = payload {
            let str_payload = core::str::from_utf8(payload).unwrap();
            edgeless_function::parse_init_payload(str_payload)
        } else {
            std::collections::HashMap::new()
        };

        let seed = arguments.get("seed").unwrap_or(&"0").parse::<u32>().unwrap_or(0);

        let is_client = arguments.get("is_client").unwrap_or(&"false").to_lowercase() == "true";
        let input_size = arguments.get("input_size").unwrap_or(&"100").parse::<usize>().unwrap_or(100);

        let _ = CONF.set(Conf { is_client, input_size });

        let mut lcg = edgeless_function::lcg::Lcg::new(seed);
        let matrix = edgeless_function::lcg::random_matrix(
            &mut lcg,
            match is_client {
                true => 0,
                false => input_size,
            },
        );

        let _ = STATE.set(std::sync::Mutex::new(State { next_id: 0, lcg, matrix }));

        if is_client {
            delayed_cast(1000, "self", b"");
        }
    }

    fn handle_stop() {
        // Noop
    }
}

edgeless_function::export!(VectorMulFunction);
