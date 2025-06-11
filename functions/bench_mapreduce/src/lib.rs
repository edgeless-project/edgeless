// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use base64::Engine;
use edgeless_function::*;

#[derive(Debug, PartialEq)]
struct Message {
    transaction_id: u32,
    source_id: u32,
    data: Vec<f32>,
}

impl Message {
    fn new(message: &[u8], use_base64: bool) -> Self {
        let bytes;
        let encoded_message = if use_base64 {
            bytes = Some(base64::engine::general_purpose::STANDARD.decode(message).unwrap());
            &bytes.unwrap()
        } else {
            message
        };
        let iter = encoded_message.chunks_exact(4);
        let mut data = vec![];
        for buf in iter {
            let mut copy = [0; 4];
            copy.copy_from_slice(buf);
            data.push(f32::from_be_bytes(copy));
        }
        Self {
            transaction_id: 0,
            source_id: 0,
            data,
        }
    }

    fn from(message: &[u8], use_base64: bool) -> Option<Self> {
        let bytes;
        let encoded_message = if use_base64 {
            bytes = Some(base64::engine::general_purpose::STANDARD.decode(message).unwrap());
            &bytes.unwrap()
        } else {
            message
        };
        let mut iter = encoded_message.chunks_exact(4);
        let transaction_id = match iter.next() {
            Some(buf) => {
                let mut copy = [0; 4];
                copy.copy_from_slice(buf);
                u32::from_be_bytes(copy)
            }
            None => {
                return None;
            }
        };
        let source_id = match iter.next() {
            Some(buf) => {
                let mut copy = [0; 4];
                copy.copy_from_slice(buf);
                u32::from_be_bytes(copy)
            }
            None => {
                return None;
            }
        };
        let mut data = vec![];
        for buf in iter {
            let mut copy = [0; 4];
            copy.copy_from_slice(buf);
            data.push(f32::from_be_bytes(copy));
        }
        Some(Self {
            transaction_id,
            source_id,
            data,
        })
    }

    fn encode(&self, use_base64: bool) -> Vec<u8> {
        let mut ret = vec![];
        ret.append(&mut self.transaction_id.to_be_bytes().to_vec());
        ret.append(&mut self.source_id.to_be_bytes().to_vec());
        ret.append(&mut self.data.iter().flat_map(|x| x.to_be_bytes()).collect::<Vec<u8>>());
        if use_base64 {
            base64::engine::general_purpose::STANDARD.encode(ret).as_bytes().to_vec()
        } else {
            ret
        }
    }
}

struct Conf {
    init_id_from_redis: bool,
    is_first: bool,
    is_last: bool,
    use_base64: bool,
    inputs: Vec<u32>,
    outputs: Vec<u32>,
}

struct State {
    transaction_id: u32,
    pending: std::collections::HashMap<u32, Message>,
}

static CONF: std::sync::OnceLock<Conf> = std::sync::OnceLock::new();
static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

/// Function that implements fan-in/fan-out of multiple sources/sinks.
///
/// It receives input from a number of sources, identified from an input
/// identifier encoded in the message, waits until all the inputs have been
/// received, and then calls all the expected outputs with an argument that is
/// the longest among the arguments received.
///
/// The function must be triggered externally, i.e., it never self-calls.
///
/// Outputs:
///
/// - `redis`: the last element saves the identifier of the last message
///   correctly received, which is read from the first element during the
///   initialization phase, if this feature is enabled
/// - `err`: errors are sent here as human-readable string messages
/// - `out-x`: the output channel to which the event is generated
///
/// Init-payload: a comma-separated list of K=V values, with the following keys:
///
/// - init_id_from_redis: true if the transaction identifier of the first
///   element is retrieved from a redis resource
/// - is_first: true if this is the first element of the workflow
/// - is_last: true if this is the last element of the workflow
/// - use_base64: true if the messages are base64-encoded/decoded
/// - inputs: colon-separated list of numerical identifiers of expected sources
/// - outputs: colon-separated list of numerical identifiers of output channels
///
struct BenchMapReduce;

impl EdgeFunction for BenchMapReduce {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        let conf = CONF.get().unwrap();
        let mut state = STATE.get().unwrap().lock().unwrap();

        // If we are the first element and this is the very first invocation,
        // then we try to read the initial transaction identifier from a
        // Redis resource, if requested
        if conf.is_first && conf.init_id_from_redis && state.transaction_id == 0 {
            state.transaction_id = match call("redis", "last_transaction_id".as_bytes()) {
                CallRet::Reply(owned_byte_buff) => core::str::from_utf8(&owned_byte_buff)
                    .unwrap_or_default()
                    .parse::<u32>()
                    .unwrap_or_default(),
                _ => 0,
            };
        }

        // Decode the message:
        // - first element: the entire payload is assumed as data input;
        // - otherwise: a structured Message is expected.
        let mut message = if conf.is_first {
            Message::new(encoded_message, conf.use_base64)
        } else if let Some(message) = Message::from(encoded_message, conf.use_base64) {
            message
        } else {
            cast(
                "err",
                format!("error: discarded invalid message casted (size {})", encoded_message.len()).as_bytes(),
            );
            return;
        };

        if conf.is_first {
            telemetry_log(5, "tbegin", &state.transaction_id.to_string());
            message.transaction_id = state.transaction_id;
            state.transaction_id += 1;
            for output in &conf.outputs {
                message.source_id = *output;
                cast(format!("out-{}", output).as_str(), &message.encode(conf.use_base64));
            }
        } else {
            // Discard the message if coming from an expected source.
            if !conf.inputs.iter().any(|x| *x == message.source_id) {
                cast(
                    "err",
                    format!(
                        "error: discarded message from unexpected source {} (expected: {})",
                        message.source_id,
                        conf.inputs.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",")
                    )
                    .as_bytes(),
                );
                return;
            }

            // Add the message to the pending ones.
            state.pending.insert(message.source_id, message);

            // Remove all messages from old transactions.
            let mut transaction_ids = state.pending.values().map(|m| m.transaction_id).collect::<Vec<u32>>();
            transaction_ids.sort_unstable();
            let last_transaction_id = *transaction_ids.last().unwrap();
            state.pending.retain(|_, m| m.transaction_id == last_transaction_id);

            // If the transaction is complete:
            // - last element: log the end of the transaction;
            // - otherwise: sum the vectors and invoke the downstream outputs.
            if state.pending.len() == conf.inputs.len() {
                if conf.is_last {
                    telemetry_log(5, "tend", &last_transaction_id.to_string());
                    if conf.init_id_from_redis {
                        cast("redis", format!("{}", last_transaction_id).as_bytes())
                    }
                } else {
                    // Reduce.
                    let mut iter = state.pending.iter_mut();
                    let first = iter.next().unwrap().1;
                    for next in iter {
                        let cur = next.1;
                        for i in 0..std::cmp::min(first.data.len(), cur.data.len()) {
                            first.data[i] += cur.data[i];
                        }
                    }

                    // Fan-out.
                    for output in &conf.outputs {
                        first.source_id = *output;
                        cast(format!("out-{}", output).as_str(), &first.encode(conf.use_base64));
                    }

                    // Clear the queue of pending messages.
                    state.pending.clear();
                }
            }
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        cast("err", "error: call handler invoked".as_bytes());
        CallRet::NoReply
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        // edgeless_function::init_logger();

        // parse initialization string
        let arguments = edgeless_function::init_payload_to_args(payload);
        let init_id_from_redis = edgeless_function::arg_to_bool("init_id_from_redis", &arguments);
        let is_first = edgeless_function::arg_to_bool("is_first", &arguments);
        let is_last = edgeless_function::arg_to_bool("is_last", &arguments);
        let use_base64 = edgeless_function::arg_to_bool("use_base64", &arguments);
        let inputs = edgeless_function::arg_to_vec::<u32>("inputs", ":", &arguments);
        let outputs = edgeless_function::arg_to_vec::<u32>("outputs", ":", &arguments);

        // check configuration errors
        if is_first && !inputs.is_empty() {
            cast("err", "init error: first element with non-empty inputs".as_bytes());
        }
        if !is_first && inputs.is_empty() {
            cast("err", "init error: non-first element with empty inputs".as_bytes());
        }
        if is_last && !outputs.is_empty() {
            cast("err", "init error: last element with non-empty outputs".as_bytes());
        }
        if !is_last && outputs.is_empty() {
            cast("err", "init error: non-last element with empty outputs".as_bytes());
        }
        if is_first && is_last {
            cast("err", "init error: element indicated as first and last".as_bytes());
        }

        // save configuration
        let _ = CONF.set(Conf {
            init_id_from_redis,
            is_first,
            is_last,
            use_base64,
            inputs,
            outputs,
        });

        let _ = STATE.set(std::sync::Mutex::new(State {
            transaction_id: 0,
            pending: std::collections::HashMap::new(),
        }));
    }

    fn handle_stop() {}
}

edgeless_function::export!(BenchMapReduce);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_message_encode_decode() {
        let message = Message {
            transaction_id: 1,
            source_id: 2,
            data: vec![1.0, 2.0, 3.0],
        };

        for use_base64 in [true, false] {
            let encoded = message.encode(use_base64);

            println!("{:?}", encoded);

            let decoded = Message::from(encoded.as_slice(), use_base64);

            println!("{:?}", decoded);
            assert_eq!(decoded.unwrap(), message);
        }
    }
}
