// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_function::*;

#[derive(Debug, PartialEq)]
struct Message {
    transaction_id: u32,
    source_id: u32,
    data: Vec<f32>,
}

impl Message {
    fn new(encoded_message: &[u8]) -> Self {
        let mut iter = encoded_message.chunks_exact(4);
        let mut data = vec![];
        loop {
            if let Some(buf) = iter.next() {
                let mut copy = [0 as u8; 4];
                copy.copy_from_slice(buf);
                data.push(f32::from_be_bytes(copy));
            } else {
                break;
            }
        }
        Self {
            transaction_id: 0,
            source_id: 0,
            data,
        }
    }

    fn from(encoded_message: &[u8]) -> Option<Self> {
        let mut iter = encoded_message.chunks_exact(4);
        let transaction_id = match iter.next() {
            Some(buf) => {
                let mut copy = [0 as u8; 4];
                copy.copy_from_slice(buf);
                u32::from_be_bytes(copy)
            }
            None => {
                return None;
            }
        };
        let source_id = match iter.next() {
            Some(buf) => {
                let mut copy = [0 as u8; 4];
                copy.copy_from_slice(buf);
                u32::from_be_bytes(copy)
            }
            None => {
                return None;
            }
        };
        let mut data = vec![];
        loop {
            if let Some(buf) = iter.next() {
                let mut copy = [0 as u8; 4];
                copy.copy_from_slice(buf);
                data.push(f32::from_be_bytes(copy));
            } else {
                break;
            }
        }
        Some(Self {
            transaction_id,
            source_id,
            data,
        })
    }

    fn encode(&self) -> Vec<u8> {
        let mut ret = vec![];
        ret.append(&mut self.transaction_id.to_be_bytes().to_vec());
        ret.append(&mut self.source_id.to_be_bytes().to_vec());
        ret.append(&mut self.data.iter().map(|x| x.to_be_bytes()).flatten().collect::<Vec<u8>>());
        ret
    }
}

struct Conf {
    is_first: bool,
    is_last: bool,
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
/// - `metric`: where the first and last elements trace the workflow latency
/// - `err`: errors are sent here as human-readable string messages
/// - `out-x`: the output channel to which the event is generated
///
/// Init-payload: a comma-separated list of K=V values, with the following keys:
///
/// - is_first: true if this is the first element of the workflow
/// - is_last: true if this is the last element of the workflow
/// - inputs: colon-separated list of numerical identifiers of expected sources
/// - outputs: colon-separated list of numerical identifiers of output channels
///
struct BenchMapReduce;

impl EdgeFunction for BenchMapReduce {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        let conf = CONF.get().unwrap();
        let mut state = STATE.get().unwrap().lock().unwrap();

        // Decode the message:
        // - first element: the entire payload is assumed as data input;
        // - otherwise: a structured Message is expected.
        let mut message = if conf.is_first {
            Message::new(encoded_message)
        } else {
            if let Some(message) = Message::from(encoded_message) {
                message
            } else {
                cast(
                    "err",
                    format!("error: discarded invalid message casted (size {})", encoded_message.len()).as_bytes(),
                );
                return;
            }
        };

        if conf.is_first {
            cast("metric", format!("workflow:begin:{}", state.transaction_id).as_bytes());
            message.transaction_id = state.transaction_id;
            state.transaction_id += 1;
            for output in &conf.outputs {
                message.source_id = *output;
                cast(format!("out-{}", output).as_str(), &message.encode());
            }
        } else {
            // Discard the message if coming from an expected source.
            if conf.inputs.iter().find(|x| **x == message.source_id).is_none() {
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
            let mut transaction_ids = state.pending.iter().map(|(_, m)| m.transaction_id).collect::<Vec<u32>>();
            transaction_ids.sort_unstable();
            let last_transaction_id = *transaction_ids.last().unwrap();
            state.pending.retain(|_, m| m.transaction_id == last_transaction_id);

            // If the transaction is complete:
            // - last element: save the end of the transaction to metric;
            // - otherwise: sum the vectors and invoke the downstream outputs.
            if state.pending.len() == conf.inputs.len() {
                if conf.is_last {
                    cast("metric", format!("workflow:end:{}", last_transaction_id).as_bytes());
                } else {
                    // Reduce.
                    let mut iter = state.pending.iter_mut();
                    let first = iter.next().unwrap().1;
                    loop {
                        if let Some(next) = iter.next() {
                            let cur = next.1;
                            for i in 0..std::cmp::min(first.data.len(), cur.data.len()) {
                                first.data[i] += cur.data[i];
                            }
                        } else {
                            break;
                        }
                    }

                    // Fan-out.
                    for output in &conf.outputs {
                        first.source_id = *output;
                        cast(format!("out-{}", output).as_str(), &first.encode());
                    }

                    // Clear the queue of pending messages.
                    state.pending.clear();
                }
            }
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        cast("err", format!("error: call handler invoked").as_bytes());
        CallRet::NoReply
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        // edgeless_function::init_logger();

        // parse initialization string
        let arguments = edgeless_function::init_payload_to_args(payload);
        let is_first = edgeless_function::arg_to_bool("is_first", &arguments);
        let is_last = edgeless_function::arg_to_bool("is_last", &arguments);
        let inputs = edgeless_function::arg_to_vec::<u32>("inputs", ":", &arguments);
        let outputs = edgeless_function::arg_to_vec::<u32>("outputs", ":", &arguments);

        // check configuration errors
        if is_first && !inputs.is_empty() {
            cast("err", format!("init error: first element with non-empty inputs").as_bytes());
        }
        if !is_first && inputs.is_empty() {
            cast("err", format!("init error: non-first element with empty inputs").as_bytes());
        }
        if is_last && !outputs.is_empty() {
            cast("err", format!("init error: last element with non-empty outputs").as_bytes());
        }
        if !is_last && outputs.is_empty() {
            cast("err", format!("init error: non-last element with empty outputs").as_bytes());
        }
        if is_first && is_last {
            cast("err", format!("init error: element indicated as first and last").as_bytes());
        }

        // save configuration
        let _ = CONF.set(Conf {
            is_first,
            is_last,
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

        let encoded = message.encode();

        println!("{:?}", encoded);

        let decoded = Message::from(encoded.as_slice());

        println!("{:?}", decoded);
        assert_eq!(decoded.unwrap(), message);
    }
}
