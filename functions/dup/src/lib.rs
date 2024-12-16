// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub use edgeless_function::*;

struct DupFunction;

impl EdgeFunction for DupFunction {
    fn handle_cast(src: InstanceId, encoded_message: &[u8]) {
        cast("out1", encoded_message);
        cast("out2", encoded_message);
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::Err
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {}

    fn handle_stop() {}
}

edgeless_function::export!(DupFunction);
