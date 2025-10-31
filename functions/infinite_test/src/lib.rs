// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_function::*;

struct InfiniteTest;

/// This is a function only for testing: its cast() method NEVER returns,
/// which leads to undefined behavior.
impl EdgeFunction for InfiniteTest {
    fn handle_cast(_src: InstanceId, _encoded_message: &[u8]) {
        let mut lcg = edgeless_function::lcg::Lcg::new(42);
        let mut cnt: u128 = 0;
        loop {
            cnt += 1;
            log::info!("{}", cnt);
            for _ in 0..1000 {
                let values = edgeless_function::lcg::random_vector(&mut lcg, 1000000);
                for value in values {
                    let _ = value.sin();
                }
            }
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();

        delayed_cast(1000, "self", &vec![]);
    }

    fn handle_stop() {}
}

edgeless_function::export!(InfiniteTest);
