// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use base64::Engine;
use edgeless_function::*;

enum OutType {
    Constant(Vec<u8>),
    Counter,
    RandVec(Vec<u8>),
}

#[derive(PartialEq, Debug)]
enum InterarrivalType {
    Constant(usize),
    Uniform(usize, usize),
    Exponential(usize),
}

struct Conf {
    out_type: OutType,
    interarrival_type: InterarrivalType,
    log: bool,
}

struct State {
    cnt: usize,
    cnt_string: Vec<u8>,
    lcg: edgeless_function::lcg::Lcg,
}

fn next_output<'a>(conf: &'a Conf, state: &'a mut State) -> &'a [u8] {
    match &conf.out_type {
        OutType::Constant(value) => value,
        OutType::Counter => {
            state.cnt_string = format!("{}", state.cnt).as_bytes().to_vec();
            state.cnt += 1;
            state.cnt_string.as_slice()
        }
        OutType::RandVec(rand_vec) => rand_vec,
    }
}

static CONF: std::sync::OnceLock<Conf> = std::sync::OnceLock::new();
static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

impl InterarrivalType {
    fn new(spec: &str) -> Result<Self, String> {
        if spec.is_empty() {
            return Err("empty interarrival".to_string());
        }
        let tokens = spec.trim().split(&['(', ')'][..]).filter(|x| !x.is_empty()).collect::<Vec<&str>>();
        if tokens.len() != 2 {
            return Err("invalid spec".to_string());
        }
        match tokens[0].to_lowercase().as_str() {
            "c" => match tokens[1].parse::<usize>() {
                Ok(period) => Ok(Self::Constant(period)),
                Err(err) => Err(format!("invalid constant period: {}", err)),
            },
            "u" => {
                let subtokens = tokens[1].split(';').collect::<Vec<&str>>();
                if subtokens.len() != 2 {
                    return Err("invalid (min,max) uniform values".to_string());
                }
                let unif_min = match subtokens[0].parse::<usize>() {
                    Ok(value) => value,
                    Err(err) => return Err(format!("invalid uniform lower bound: {}", err)),
                };
                let unif_max = match subtokens[1].parse::<usize>() {
                    Ok(value) => value,
                    Err(err) => return Err(format!("invalid uniform upper bound: {}", err)),
                };
                if unif_min >= unif_max {
                    return Err(format!("invalid bounds: {} >= {}", unif_min, unif_max));
                }
                Ok(Self::Uniform(unif_min, unif_max))
            }
            "e" => match tokens[1].parse::<usize>() {
                Ok(mean) => Ok(Self::Exponential(mean)),
                Err(err) => Err(format!("invalid exponential mean: {}", err)),
            },
            _ => Err(format!("unknown interarrival type {}", tokens[0])),
        }
    }

    /// Return the next arrival time, in ms.
    fn next(&self, lcg: &mut edgeless_function::lcg::Lcg) -> usize {
        match self {
            InterarrivalType::Constant(t) => *t,
            InterarrivalType::Uniform(a, b) => *a + ((*b - *a) as f32 * lcg.rand()).round() as usize,
            InterarrivalType::Exponential(m) => {
                let mut rnd = lcg.rand();
                if !rnd.is_normal() {
                    rnd = f32::MIN_POSITIVE;
                }
                (-f32::ln(rnd) * *m as f32).round() as usize
            }
        }
    }
}

/// Function that generates different types of events based on an arrival model.
///
/// Outputs:
///
/// - `out`: the output channel to which the event is generated
///
/// Init-payload: a comma-separated list of K=V values, with the following keys:
///
/// - out_type: one of constant, counter, rand_vec
/// - size: [rand_vec] the size of the vector
/// - value: [constant] value produced as output
/// - seed: pseudo-random number seed initializer
/// - arrival: one of
///   - c(T): constant interarrival with period equal to T ms
///   - u(A;B): uniformly distributed interarrival between A ms and B ms
///   - e(M): exponentially distributed interarrival with average M ms
/// - log: Boolean flag, if out_type=counter and the flag is true, create
///   telemetry_log events tbegin, or tend, when a message is generated, or
///   received, respectively. It can be used to compute RTT delay by
///   post-processing the timestamps of tbegin/tend events.
///
struct Trigger;

impl EdgeFunction for Trigger {
    fn handle_cast(_src: InstanceId, msg: &[u8]) {
        let conf = CONF.get().unwrap();
        if msg.is_empty() {
            // New message to be created.

            let mut state = STATE.get().unwrap().lock().unwrap();

            // Create an event towards the next function instance.
            let cur_counter = state.cnt;
            cast("out", next_output(conf, &mut state));

            // Log the emission of a new message.
            if conf.log {
                telemetry_log(5, "tbegin", &cur_counter.to_string());
            }

            // Schedule the next event to be generated.
            delayed_cast(conf.interarrival_type.next(&mut state.lcg) as u64, "self", &[]);
        } else {
            // Message received back.
            if conf.log {
                telemetry_log(5, "tend", core::str::from_utf8(msg).unwrap_or_default());
            }
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        // edgeless_function::init_logger();

        let arguments = edgeless_function::init_payload_to_args(payload);

        let use_base64 = edgeless_function::arg_to_bool("use_base64", &arguments);

        let seed = arguments.get("seed").unwrap_or(&"0").parse::<u32>().unwrap_or(0);
        let mut lcg = edgeless_function::lcg::Lcg::new(seed);

        let out_type = match arguments.get("out_type") {
            Some(out_type) => match *out_type {
                "constant" => match arguments.get("value") {
                    Some(value) => Ok(OutType::Constant(value.as_bytes().to_vec())),
                    None => Err("Missing value".to_string()),
                },
                "counter" => Ok(OutType::Counter),
                "rand_vec" => match arguments.get("size") {
                    Some(size) => match size.parse::<usize>() {
                        Ok(size) => {
                            let rnd_vec = edgeless_function::lcg::random_vector(&mut lcg, size);
                            let rand_vec = rnd_vec.iter().map(|x| x.to_be_bytes()).flatten().collect::<Vec<u8>>();
                            let bytes = if use_base64 {
                                base64::engine::general_purpose::STANDARD.encode(rand_vec).as_bytes().to_vec()
                            } else {
                                rand_vec
                            };
                            Ok(OutType::RandVec(bytes))
                        }
                        Err(err) => Err(format!("{}", err)),
                    },
                    None => Err("Missing size".to_string()),
                },
                _ => Err(format!("Unknown out_type {}", out_type)),
            },
            None => Err("Missing out_type".to_string()),
        };

        let out_type = match out_type {
            Ok(val) => val,
            Err(err) => {
                cast("err", format!("Invalid output config: {}", err).as_bytes());
                return;
            }
        };

        let interarrival_type = match InterarrivalType::new(arguments.get("arrival").unwrap_or(&"")) {
            Ok(val) => val,
            Err(err) => {
                cast("err", format!("Invalid inter-arrival config: {}", err).as_bytes());
                return;
            }
        };

        let log = matches!(out_type, OutType::Counter) && arg_to_bool("log", &arguments);

        let _ = CONF.set(Conf {
            out_type,
            interarrival_type,
            log,
        });
        let _ = STATE.set(std::sync::Mutex::new(State {
            cnt: 0,
            cnt_string: vec![],
            lcg,
        }));

        cast("self", &[]);
    }

    fn handle_stop() {}
}

edgeless_function::export!(Trigger);

#[cfg(test)]
mod tests {
    use super::InterarrivalType;

    #[test]
    fn test_trigger_interarrival_type_new() {
        assert!(InterarrivalType::new("").is_err());
        assert!(InterarrivalType::new("invalid").is_err());
        assert!(InterarrivalType::new("c()").is_err());
        assert!(InterarrivalType::new("c(A)").is_err());
        assert!(InterarrivalType::new("c(1,2)").is_err());
        assert!(InterarrivalType::new("c(3.14)").is_err());
        assert!(InterarrivalType::new("u()").is_err());
        assert!(InterarrivalType::new("u(A)").is_err());
        assert!(InterarrivalType::new("u(1)").is_err());
        assert!(InterarrivalType::new("u(1;2;3)").is_err());
        assert!(InterarrivalType::new("u(1.0;2.0)").is_err());
        assert!(InterarrivalType::new("u(1;1)").is_err());
        assert!(InterarrivalType::new("u(2;1)").is_err());
        assert!(InterarrivalType::new("e()").is_err());
        assert!(InterarrivalType::new("e(A)").is_err());
        assert!(InterarrivalType::new("e(A,B)").is_err());
        assert!(InterarrivalType::new("e(1,2)").is_err());
        assert!(InterarrivalType::new("e(3.14)").is_err());

        assert_eq!(InterarrivalType::Constant(0), InterarrivalType::new("c(0)").unwrap());
        assert_eq!(InterarrivalType::Constant(0), InterarrivalType::new(" c(0)").unwrap());
        assert_eq!(InterarrivalType::Constant(0), InterarrivalType::new("  c(0)\t").unwrap());
        assert_eq!(InterarrivalType::Constant(0), InterarrivalType::new(" c(0)\n\n").unwrap());
        assert_eq!(InterarrivalType::Constant(999), InterarrivalType::new("c(999)").unwrap());

        assert_eq!(InterarrivalType::Uniform(0, 1), InterarrivalType::new("u(0;1)").unwrap());
        assert_eq!(InterarrivalType::Uniform(1, 999), InterarrivalType::new("u(1;999)").unwrap());

        assert_eq!(InterarrivalType::Exponential(0), InterarrivalType::new("e(0)").unwrap());
        assert_eq!(InterarrivalType::Exponential(999), InterarrivalType::new("e(999)").unwrap());
    }

    #[test]
    fn test_trigger_interarrival_type_next() {
        let mut lcg = edgeless_function::lcg::Lcg::new(0);

        let constant = InterarrivalType::Constant(42);
        for _ in 0..10 {
            assert_eq!(42, constant.next(&mut lcg));
        }

        let uniform = InterarrivalType::Uniform(100, 200);
        let mut values = std::collections::HashSet::new();
        for _ in 0..10 {
            let x = uniform.next(&mut lcg);
            assert!(x >= 100 && x <= 200);
            values.insert(x);
        }
        assert_eq!(10, values.len());

        let exponential = InterarrivalType::Exponential(100);
        values.clear();
        for _ in 0..10 {
            let x = exponential.next(&mut lcg);
            assert!(x >= 1 && x <= 200);
            values.insert(x);
        }
        assert_eq!(10, values.len());
    }
}
