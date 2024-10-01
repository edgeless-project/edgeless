// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;
struct PongerFun;

impl EdgeFunction for PongerFun {
    fn handle_cast(src: InstanceId, encoded_message: &[u8]) {
        log::info!("AsyncPonger: 'Cast' called, MSG: {:?}", encoded_message);
        cast_raw(src, b"PONG2");
        //OR cast("pinger", b"PONG2");
    }

    fn handle_call(_src: InstanceId, encoded_message: &[u8]) -> CallRet {
        log::info!("AsyncPonger: 'Call' called, MSG: {:?}", encoded_message);
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("AsyncPonger: 'Init' called");
    }

    fn handle_stop() {
        log::info!("AsyncPonger: 'Stop' called");
    }
}

edgeless_function::export!(PongerFun);

