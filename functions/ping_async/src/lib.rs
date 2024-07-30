// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct PingerFun;

impl EdgeFunction for PingerFun {
    fn handle_cast(_src: InstanceId, port: &str, encoded_message: &[u8]) {
        let msg = core::str::from_utf8(encoded_message);

        if msg.unwrap() == "wakeup" {
            log::info!("AsyncPinger: 'Cast' Wakeup");
            cast("ping", b"PING");
            delayed_cast(1000, "self", b"wakeup");
        } else {
            log::info!("AsyncPinger: 'Cast' Got Response");
        }
    }

    fn handle_call(_src: InstanceId, _port: &str, encoded_message: &[u8]) -> CallRet {
        log::info!("AsyncPinger: 'Call' called, MSG: {}", core::str::from_utf8(encoded_message).unwrap());
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("AsyncPinger: 'Init' called");
        delayed_cast(10000, "self", b"wakeup");
    }

    fn handle_stop() {
        log::info!("AsyncPinger: 'Stop' called");
    }
}

edgeless_function::export!(PingerFun);
