// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;
struct PongerFun;

struct PongType {
    msg: String,
}

#[derive(Debug)]
struct PingType {
    msg: String,
}

impl edgeless_function_core::Deserialize for PingType {
    fn deserialize(data: &[u8]) -> Self {
        PingType {
            msg: String::from_utf8(data.to_vec()).unwrap(),
        }
    }
}

impl edgeless_function_core::Serialize for PongType {
    fn serialize(&self) -> Vec<u8> {
        self.msg.as_bytes().to_vec()
    }
}

edgeless_function::generate!(PongerFun);

impl PongAsyncAPI for PongerFun {
    type EDGELESS_EXAMPLE_PING = PingType;
    type EDGELESS_EXAMPLE_PONG = PongType;

    fn handle_cast_ping(src: InstanceId, ping_msg: PingType) {
        log::info!("AsyncPonger: 'Cast' called, MSG: {:?}", ping_msg);
        cast_pong(&PongType { msg: "PONG2".to_string() });
    }

    fn handle_internal(_msg: &[u8]) {
        // NOOP
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("AsyncPonger: 'Init' called");
    }

    fn handle_stop() {
        log::info!("AsyncPonger: 'Stop' called");
    }
}
