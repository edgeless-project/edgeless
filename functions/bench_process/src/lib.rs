// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_function::*;

struct Conf {
    forward: bool,
    fibonacci: u64,
    _memory: Vec<u8>,
}

static CONF: std::sync::OnceLock<Conf> = std::sync::OnceLock::new();

// from: https://docs.rs/num-bigint/0.4.6/num_bigint/
fn fibonacci_n_th_element(n: u64) -> num_bigint::BigUint {
    let mut f0 = num_bigint::BigUint::ZERO;
    let mut f1 = num_bigint::BigUint::from(1_u64);
    for _ in 0..n {
        let f2 = f0 + &f1;
        f0 = f1;
        f1 = f2;
    }
    f0
}

/// Function that emulates processing by doing some predefined operations.
///
/// Outputs:
///
/// - `out`: the output channel to which the event is generated
///
/// Init-payload: a comma-separated list of K=V values, with the following keys:
///
/// - forward: if true, forward the message received on the `out` channel output
/// - fibonacci: the n-th element of the Fibonacci sequence to be computed
/// - allocate: the amount of memory to be allocated, in bytes
///
struct BenchProcess;

impl EdgeFunction for BenchProcess {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        let conf = CONF.get().unwrap();

        fibonacci_n_th_element(conf.fibonacci);

        if conf.forward {
            cast("out", encoded_message);
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        // edgeless_function::init_logger();

        let arguments = edgeless_function::init_payload_to_args(payload);
        let forward = edgeless_function::arg_to_bool("forward", &arguments);
        let fibonacci = arguments.get("fibonacci").unwrap_or(&"0").parse::<u64>().unwrap_or(0);
        let allocate = arguments.get("allocate").unwrap_or(&"0").parse::<usize>().unwrap_or(0);

        let _ = CONF.set(Conf {
            forward,
            fibonacci,
            _memory: Vec::with_capacity(allocate),
        });
    }

    fn handle_stop() {}
}

edgeless_function::export!(BenchProcess);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fibonacci_n_th_element() {
        let expected: Vec<num_bigint::BigUint> = vec![
            num_bigint::BigUint::ZERO,
            num_bigint::BigUint::from(1_u64),
            num_bigint::BigUint::from(1_u64),
            num_bigint::BigUint::from(2_u64),
            num_bigint::BigUint::from(3_u64),
            num_bigint::BigUint::from(5_u64),
            num_bigint::BigUint::from(8_u64),
            num_bigint::BigUint::from(13_u64),
            num_bigint::BigUint::from(21_u64),
            num_bigint::BigUint::from(34_u64),
            num_bigint::BigUint::from(55_u64),
            num_bigint::BigUint::from(89_u64),
            num_bigint::BigUint::from(144_u64),
            num_bigint::BigUint::from(233_u64),
            num_bigint::BigUint::from(377_u64),
            num_bigint::BigUint::from(610_u64),
            num_bigint::BigUint::from(987_u64),
            num_bigint::BigUint::from(1597_u64),
            num_bigint::BigUint::from(2584_u64),
            num_bigint::BigUint::from(4181_u64),
        ];
        for n in 0..expected.len() {
            assert_eq!(expected[n], fibonacci_n_th_element(n as u64));
        }
    }

    #[test]
    #[ignore]
    fn test_fibonacci_n_th_element_single() {
        fibonacci_n_th_element(std::env::var("N").unwrap().parse::<u64>().unwrap());
    }
}
