// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;
struct PongerFun;

impl EdgeFunction for PongerFun {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        log::info!("Ponger: 'Cast' called, MSG: {:?}", encoded_message);
    }

    fn handle_call(_src: InstanceId, encoded_message: &[u8]) -> CallRet {
        log::info!("Ponger: 'Call' called, MSG: {:?}", encoded_message);
        CallRet::Reply(OwnedByteBuff::new_from_slice(b"PONG"))
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Ponger: 'Init' called");
    }

    fn handle_stop() {
        log::info!("Ponger: 'Stop' called");
    }
}
edgeless_function::export!(PongerFun);
