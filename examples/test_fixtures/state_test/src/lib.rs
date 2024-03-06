// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_function::*;
use log;

struct StateTest;

impl EdgeFunction for StateTest {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        match core::str::from_utf8(encoded_message).unwrap() {
            "test_cast_raw_output" => {
                sync("new_state".as_bytes());
            }
            _ => {
                log::info!("Unprocessed Message");
            }
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        if let Some(state) = serialized_state {
            let data = core::str::from_utf8(state).unwrap();
            // no-std log does not support parameters
            telemetry_log(3, "edgeless_test_state", data);
        } else {
            log::info!("no_state");
        }
    }

    fn handle_stop() {}
}

edgeless_function::export!(StateTest);
