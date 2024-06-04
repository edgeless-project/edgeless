// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
pub use edgeless_function::*;

#[derive(Debug, Default)]
struct NoopFunction;

impl EdgeFunction for NoopFunction {
    fn handle_cast(src: InstanceId, encoded_message: &[u8]) {
        log::info!(
            "Noop casted, node {:?}, function {:?}, MSG: {:?}",
            src.node_id,
            src.component_id,
            encoded_message
        );
    }

    fn handle_call(src: InstanceId, encoded_message: &[u8]) -> CallRet {
        log::info!(
            "Noop called, node {:?}, function {:?}, MSG: {:?}",
            src.node_id,
            src.component_id,
            encoded_message
        );
        CallRet::NoReply
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Noop initialized, payload: {:?}", payload);
    }

    fn handle_stop() {
        log::info!("Noop stopped");
    }
}

//#[cfg(target_arch = "wasm")]
//edgeless_function::export!(NoopFunction);

//#[cfg(target_arch = "x86_64")]
//edgeless_function::export_x86!(NoopFunction, NoopFunction::default);

//#[cfg(target_arch = "x86_64")]
edgeless_function::export_x86a!(NoopFunction);