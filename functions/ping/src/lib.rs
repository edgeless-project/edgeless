// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use std::ops::Deref;

use edgeless_function::*;

struct PingerFun;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct PingerState {
    count: u64,
}

static STATE: std::sync::OnceLock<std::sync::Mutex<PingerState>> = std::sync::OnceLock::new();

impl EdgeFunction for PingerFun {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        let str_message = core::str::from_utf8(encoded_message).unwrap();
        log::info!("Pinger: 'Cast' called, MSG: {}", str_message);
        if str_message == "wakeup" {
            let id = STATE.get().unwrap().lock().unwrap().count;

            STATE.get().unwrap().lock().unwrap().count += 1;
            //sync(&serde_json::to_string(STATE.get().unwrap().lock().unwrap().deref()).unwrap().as_bytes());

            let res = call("ponger", &format!("PING-{}", id).as_bytes());
            if let CallRet::Reply(_msg) = res {
                log::info!("Got Reply");
            }

            delayed_cast(1000, "self", b"wakeup");
        }
    }

    fn handle_call(_src: InstanceId, encoded_message: &[u8]) -> CallRet {
        log::info!("Pinger: 'Call' called, MSG: {:?}", encoded_message);
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Pinger: 'Init' called");
        
        STATE.set(std::sync::Mutex::new(PingerState { count: 0})).unwrap();
        /*if let Some(serialized) = serialized_state {
            STATE
                .set(std::sync::Mutex::new(
                    serde_json::from_str(core::str::from_utf8(serialized).unwrap()).unwrap(),
                ))
                .unwrap();
        } else {
            STATE.set(std::sync::Mutex::new(PingerState { count: 0 })).unwrap();
        }*/

        cast("self", b"wakeup");
    }

    fn handle_stop() {
        log::info!("Pinger: 'Stop' called");
    }
}

#[cfg(target_arch = "wasm32")]
edgeless_function::export!(PingerFun);

#[cfg(target_arch = "x86_64")]
edgeless_function::export_x86a!(PingerFun);
