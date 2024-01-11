// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::api::*;

struct PingerFun;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct PingerState {
    count: u64,
}

impl Edgefunction for PingerFun {
    fn handle_cast(_src: InstanceId, encoded_message: String) {
        log::info!("AsyncPinger: 'Cast' called, MSG: {}", encoded_message);
        if encoded_message == "wakeup" {
            cast("ponger", "PING");
            delayed_cast(1000, "self", "wakeup");
        } else {
            log::info!("Got Response");
        }
    }

    fn handle_call(_src: InstanceId, encoded_message: String) -> CallRet {
        log::info!("AsyncPinger: 'Call' called, MSG: {}", encoded_message);
        CallRet::Noreply
    }

    fn handle_init(_payload: String, serialized_state: Option<String>) {
        edgeless_function::init_logger();
        log::info!("AsyncPinger: 'Init' called");
        cast("self", "wakeup");
    }

    fn handle_stop() {
        log::info!("AsyncPinger: 'Stop' called");
    }
}

edgeless_function::export!(PingerFun);
