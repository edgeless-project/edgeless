// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct PingerFun;

edgeless_function::generate!(PingerFun);

struct PongType {
    msg: String,
}

struct PingType {
    msg: String,
}

impl edgeless_function_core::Deserialize for PongType {
    fn deserialize(data: &[u8]) -> Self {
        PongType {
            msg: String::from_utf8(data.to_vec()).unwrap(),
        }
    }
}

impl edgeless_function_core::Serialize for PingType {
    fn serialize(&self) -> Vec<u8> {
        self.msg.as_bytes().to_vec()
    }
}

impl PingAsyncAPI for PingerFun {
    type EDGELESS_EXAMPLE_PONG = PongType;
    type EDGELESS_EXAMPLE_PING = PingType;

    fn handle_cast_pong(_src: InstanceId, data: PongType) {
        log::info!("AsyncPinger: 'Cast' Got Response");
    }

    fn handle_internal(data: &[u8]) {
        log::info!("AsyncPinger: 'Cast' Wakeup");
        cast_ping(&PingType { msg: "PING".to_string() });
        delayed_cast(1000, "self", b"wakeup");
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
