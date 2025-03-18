// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
pub use edgeless_function::*;

#[derive(Debug, Default)]
struct NoopFunction;

impl EdgeFunction for NoopFunction {
    fn handle_cast(src: InstanceId, encoded_message: &[u8]) {
        println!(
            "Noop casted, node {:?}, function {:?}, MSG: {:?}",
            src.node_id,
            src.component_id,
            encoded_message
        );

        log::info!(
            "Noop casted, node {:?}, function {:?}, MSG: {:?}",
            src.node_id,
            src.component_id,
            encoded_message
        );
    }

    fn handle_call(src: InstanceId, encoded_message: &[u8]) -> CallRet {
        println!(
            "Noop called, node {:?}, function {:?}, MSG: {:?}",
            src.node_id,
            src.component_id,
            encoded_message
        );

        log::info!(
            "Noop called, node {:?}, function {:?}, MSG: {:?}",
            src.node_id,
            src.component_id,
            encoded_message
        );
        CallRet::NoReply
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        println!("Noop initialized, payload: {:?}", payload);
        edgeless_function::init_logger();
        log::info!("Noop initialized, payload: {:?}", payload);
        let id = slf();
        println!("Noop id: {}", id);
        //log::info!("Noop log id: {}", std::str::from_utf8(&id.node_id).unwrap());
        telemetry_log(1, "slf", "Noop telemtry log");
    }

    fn handle_stop() {
        println!("Noop stopped");
        log::info!("Noop stopped");
    }
}

#[cfg(target_arch = "wasm32")]
edgeless_function::export!(NoopFunction);

//#[cfg(target_arch = "x86_64")]
//edgeless_function::export_x86!(NoopFunction, NoopFunction::default);

#[cfg(target_arch = "x86_64")]
edgeless_function::export_x86a!(NoopFunction);
