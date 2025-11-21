// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use std::process::id;

use dda;
use edgeless_function::*;

struct DDAChainMid;

// ..........workflow starts
//     ┌─────────┐
//     │  First  │
//     │  func.  │
//     └────┬────┘
//          │
//          ▼
//     ┌────────────┐
//     │ Fan-out to │
//     │chain funcs │
//     │ 1..n blocks│
//     └─┬─┬─┬─┬─┬─-┘
//       │ │ │ │ │
//       ▼ ▼ ▼ ▼ ▼
//     ┌─┐┌─┐┌─┐┌─┐┌─┐
//     │1││2││3││4││n│
//     │ ││ ││ ││ ││ │
//     └─┘└─┘└─┘└─┘└─┘
//       │ │ │ │ │
//       └─┼─┼─┼─┘
//         └─┼─┘
//           ▼
//     ┌─────────┐
//     │ Fan-in  │
//     │  Block  │
//     └─────────┘
// ..........workflow ends
impl EdgeFunction for DDAChainMid {
    fn handle_cast(_source: InstanceId, encoded_message: &[u8]) {
        // middle function
        // identifier is sent as encoded_message
        let identifier_str = core::str::from_utf8(encoded_message).unwrap();
        telemetry_log(5, "function:start", identifier_str);

        // make a dda call
        match dda::publish_action("actor", vec![]) {
            Ok(action_result) => log::debug!("action publish successful"),
            Err(e) => log::error!("action publish failed"),
        }
        telemetry_log(5, "function:end", identifier_str);

        // cast to the final, fan-in block
        cast("out", identifier_str.as_bytes());
    }

    fn handle_call(_source: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
    }

    fn handle_stop() {}
}

edgeless_function::export!(DDAChainMid);
