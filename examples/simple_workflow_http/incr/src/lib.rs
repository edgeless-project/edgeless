// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::api::*;

struct IncrFun;

impl Edgefunction for IncrFun {
    fn handle_cast(_src: InstanceId, encoded_message: String) {
        log::info!("incr: called with '{}'", encoded_message);

        if let Ok(n) = encoded_message.parse::<i32>() {
            cast("result", format!("{}", n + 1).as_str());
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: String) -> CallRet {
        CallRet::Noreply
    }

    fn handle_init(_payload: String, _init_state: Option<String>) {
        edgeless_function::init_logger();
        log::info!("incr: started");
    }

    fn handle_stop() {
        log::info!("incr: stopped");
    }
}

edgeless_function::export!(IncrFun);
