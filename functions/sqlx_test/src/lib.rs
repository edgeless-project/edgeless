// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct Sqlx_test;

impl EdgeFunction for Sqlx_test {
    fn handle_cast(_src: InstanceId, message: &[u8]) {
        log::info!("-----handle cast");
        println!("sqlx cast");
    }

    fn handle_call(_src: InstanceId, _message: &[u8]) -> CallRet {
        log::info!("-----handle call");
        CallRet::NoReply
    }

    fn handle_init(init_message: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("-----handle init");
        cast("output", b" hi sqlx this is test");
    }

    fn handle_stop() {
        log::info!("-----handle stop");
        // noop
    }
}

edgeless_function::export!(Sqlx_test);
