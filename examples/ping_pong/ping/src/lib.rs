// SPDX-FileCopyrightText: © 2023 TUM
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use std::ops::Deref;

use edgeless_function::api::*;

struct PingerFun;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct PingerState {
    count: u64,
}

static STATE: std::sync::OnceLock<std::sync::Mutex<PingerState>> = std::sync::OnceLock::new();

impl Edgefunction for PingerFun {
    fn handle_cast(_src: InstanceId, encoded_message: String) {
        log::info!("Pinger: 'Cast' called, MSG: {}", encoded_message);
        if encoded_message == "wakeup" {
            let id = STATE.get().unwrap().lock().unwrap().count;

            STATE.get().unwrap().lock().unwrap().count += 1;
            sync(&serde_json::to_string(STATE.get().unwrap().lock().unwrap().deref()).unwrap());

            let res = call("ponger", &format!("PING-{}", id));
            if let CallRet::Reply(_msg) = res {
                log::info!("Got Reply");
            }

            delayed_cast(1000, "self", "wakeup");
        }
    }

    fn handle_call(_src: InstanceId, encoded_message: String) -> CallRet {
        log::info!("Pinger: 'Call' called, MSG: {}", encoded_message);
        CallRet::Noreply
    }

    fn handle_init(_payload: String, serialized_state: Option<String>) {
        edgeless_function::init_logger();
        log::info!("Pinger: 'Init' called");

        if let Some(serialized) = serialized_state {
            STATE.set(std::sync::Mutex::new(serde_json::from_str(&serialized).unwrap())).unwrap();
        } else {
            STATE.set(std::sync::Mutex::new(PingerState { count: 0 })).unwrap();
        }

        cast("self", "wakeup");
    }

    fn handle_stop() {
        log::info!("Pinger: 'Stop' called");
    }
}

edgeless_function::export!(PingerFun);
