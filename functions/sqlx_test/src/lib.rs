// SPDX-FileCopyrightText: © 2024 Chen Chen <cc2181@cam.ac.uk>
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct Sqlx_test;

impl EdgeFunction for Sqlx_test {
    fn handle_cast(_src: InstanceId, message: &[u8]) {
        edgeless_function::init_logger();
    }

    fn handle_call(_src: InstanceId, _message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(init_message: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("sqlx function init");
       
        let res = call("database", b"INSERT INTO WorkflowState (id, metadata) Values($1, '{\"foo\":1,\"bar\":\"first\"}')");
        log::info!("call keep running");
        call("database", b"SELECT id, metadata FROM WorkflowState WHERE id=$1");
        call("database", b"UPDATE WorkflowState SET metadata='{\"foo\":2,\"bar\":\"second\"}'  WHERE id = $1");
        call("database", b"DELETE FROM WorkflowState WHERE id=$1");
        
        //why is msg u8 not owneddatabyte?
        if let CallRet::Reply(msg) = res {
            if let Ok(msg) = std::str::from_utf8(&msg) {
                log::info!("Got Reply: {}", msg);
            }
            else{
                log::info!("State management reponse not ok");
            }
        }

    }

    fn handle_stop() {
        // sqlx
    }
}

edgeless_function::export!(Sqlx_test);
